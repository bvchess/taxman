#!/usr/bin/env python3
"""Tests of run_tests.py on a toy crate written to a temporary directory: its tests, ignored tests, a failing test, a
doc test, an integration test and a binary's unit test.  They build the toy crate with the test profile (no
`--release`).

usage: python3 -I test_run_tests.py      (needs cargo; a few seconds each)
"""
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCRIPT = HERE.parent / "run_tests.py"

CARGO_TOML = """[package]
name = "toy"
version = "0.1.0"
edition = "2021"

[features]
extra = []

[dependencies]
"""

LIB_RS = """/// ```
/// assert_eq!(toy::double(2), 4);
/// ```
pub fn double(x: u32) -> u32 {
    x * 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles() {
        assert_eq!(double(2), 4);
    }

    #[test]
    fn slow() {
        std::thread::sleep(std::time::Duration::from_millis(1200));
    }

    #[test]
    #[ignore]
    fn ignored() {
        panic!("an ignored test is not run");
    }

    #[test]
    fn fails_only_when_told() {
        assert!(std::env::var("TOY_FAIL").is_err(), "told to fail");
    }

    #[cfg(feature = "extra")]
    #[test]
    fn only_with_the_feature() {}

    macro_rules! cheap {
        ($($name:ident)*) => { $(#[test] fn $name() {})* };
    }

    // Eight tests that cost nothing: with the other tests of the crate there are more than ten, and the summary has
    // them all.
    cheap!(cheap_a cheap_b cheap_c cheap_d cheap_e cheap_f cheap_g cheap_h);
}
"""

MAIN_RS = """fn main() {}

#[cfg(test)]
mod tests {
    #[test]
    fn the_binary_has_a_test() {}
}
"""

IT_RS = """#[test]
fn integration_one() {
    assert_eq!(toy::double(3), 6);
}

#[test]
fn integration_runs_in_the_crate_directory() {
    assert!(std::path::Path::new("Cargo.toml").is_file());
}
"""


def load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class Toy(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.crate = Path(cls.tmp.name) / "toy"
        (cls.crate / "src").mkdir(parents=True)
        (cls.crate / "tests").mkdir()
        (cls.crate / "Cargo.toml").write_text(CARGO_TOML)
        (cls.crate / "src" / "lib.rs").write_text(LIB_RS)
        (cls.crate / "src" / "main.rs").write_text(MAIN_RS)
        (cls.crate / "tests" / "it.rs").write_text(IT_RS)

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def run_suite(self, *options, env=None):
        out = Path(self.tmp.name)
        command = [sys.executable, "-I", str(SCRIPT), "--jobs", "4", "--report", str(out / "report.txt"),
                   "--summary", str(out / "summary.json"), *options, f"solver={self.crate}"]
        done = subprocess.run(command, capture_output=True, text=True, env=env)
        summary = json.loads((out / "summary.json").read_text()) if (out / "summary.json").exists() else None
        report = (out / "report.txt").read_text() if (out / "report.txt").exists() else ""
        for name in ("summary.json", "report.txt"):
            (out / name).unlink(missing_ok=True)
        return done, summary, report

    def test_a_passing_suite_counts_every_kind_of_test(self):
        done, summary, report = self.run_suite()
        self.assertEqual(done.returncode, 0, done.stderr)
        # lib: doubles, slow, fails_only_when_told, 8 cheap ones; bin: 1; it.rs: 2; the doc test: 1 (15).  One ignored.
        self.assertEqual(summary["crates"], {"solver": {"passed": 15, "failed": 0, "ignored": 1}}, done.stderr)
        self.assertIn("solver: 15 passed, 0 failed, 1 ignored", report)

    def test_a_feature_is_given_to_the_crate_it_names_and_to_no_other(self):
        done, summary, _ = self.run_suite("--features", "solver=extra")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(summary["crates"]["solver"]["passed"], 16)  # the test that only the feature builds is one more
        done, summary, _ = self.run_suite("--features", "prover=extra")
        self.assertEqual(done.returncode, 2, done.stderr)  # there is no such crate in this run
        self.assertIsNone(summary)

    def test_a_failing_test_is_collected_and_does_not_stop_the_others(self):
        done, summary, report = self.run_suite(env={**os.environ, "TOY_FAIL": "1"})
        self.assertEqual(done.returncode, 1, done.stderr)
        self.assertEqual(summary["crates"]["solver"]["failed"], 1)
        self.assertEqual(summary["crates"]["solver"]["passed"], 14)
        self.assertIn("solver/lib:toy/tests::fails_only_when_told", report)
        self.assertIn("told to fail", report)
        self.assertIn("fails alone too", report)

    def test_the_times_file_orders_the_next_run_and_leaves_failures_out(self):
        times = Path(self.tmp.name) / "times.json"
        times.unlink(missing_ok=True)
        done, summary, _ = self.run_suite("--times", str(times), env={**os.environ, "TOY_FAIL": "1"})
        recorded = json.loads(times.read_text())["tests"]
        self.assertIn("solver/lib:toy/tests::slow", recorded)
        self.assertGreaterEqual(recorded["solver/lib:toy/tests::slow"], 1.0)
        self.assertNotIn("solver/lib:toy/tests::fails_only_when_told", recorded)
        self.assertNotIn("solver/lib:toy/tests::ignored", recorded)

    def test_a_filter_runs_only_the_matching_tests_and_keeps_the_other_times(self):
        times = Path(self.tmp.name) / "times-filter.json"
        times.write_text(json.dumps({"tests": {"solver/other:gone/old": 5.0}}))
        done, summary, _ = self.run_suite("--filter", "integration_one", "--times", str(times))
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(summary["crates"]["solver"]["passed"], 1)
        recorded = json.loads(times.read_text())["tests"]
        self.assertEqual(recorded["solver/other:gone/old"], 5.0)
        self.assertIn("solver/test:it/integration_one", recorded)

    def test_a_timeout_kills_the_test_and_fails_the_suite(self):
        done, summary, report = self.run_suite("--filter", "slow", "--timeout", "0.3")
        self.assertEqual(done.returncode, 1, done.stderr)
        self.assertIn("TIMED OUT", report)

    def test_the_summary_has_the_cpu_of_every_test_the_costliest_first_and_a_wait_is_no_cpu(self):
        done, summary, report = self.run_suite()
        self.assertEqual(done.returncode, 0, done.stderr)
        costs = {test["id"]: test for test in summary["costs"]}
        passed = summary["crates"]["solver"]["passed"]
        self.assertGreater(passed, 10)  # more tests than a short list of the costliest would hold
        self.assertEqual((len(summary["costs"]), len(costs)), (passed, passed))  # every test once
        self.assertIn("solver/lib:toy/tests::cheap_h", costs)  # a test of no cost is there too
        self.assertEqual(set(costs["solver/lib:toy/tests::slow"]), {"id", "seconds", "cpu"})
        slow = costs["solver/lib:toy/tests::slow"]  # sleeps for 1.2 s
        self.assertGreaterEqual(slow["seconds"], 1.2)
        self.assertLess(slow["cpu"], 0.5)
        self.assertEqual(summary["costs"], sorted(summary["costs"], key=lambda t: -t["cpu"]))
        self.assertIn("most CPU: ", report)

    def test_a_crate_that_does_not_build_is_an_error_of_its_own(self):
        broken = Path(self.tmp.name) / "broken"
        (broken / "src").mkdir(parents=True)
        (broken / "Cargo.toml").write_text(CARGO_TOML.replace("toy", "broken"))
        (broken / "src" / "lib.rs").write_text("pub fn f() { this does not compile }\n")
        done = subprocess.run([sys.executable, "-I", str(SCRIPT), f"solver={broken}"], capture_output=True, text=True)
        self.assertEqual(done.returncode, 2, done.stderr)


class Scheduling(unittest.TestCase):
    def test_unknown_tests_go_first_and_known_ones_longest_first(self):
        module = load("run_tests", SCRIPT)
        binary = module.Binary("solver", Path("."), "lib", "toy", "toy")
        tests = [module.Test(name, binary, name) for name in ("a", "b", "c", "d", "e")]
        order = module.schedule(tests, {"a": 1.0, "c": 30.0, "d": 5.0})
        self.assertEqual([t.id for t in order], ["b", "e", "c", "d", "a"])


class Measuring(unittest.TestCase):
    def test_a_command_that_computes_uses_cpu_and_one_that_sleeps_does_not(self):
        module = load("run_tests", SCRIPT)
        spin = "import time; t = time.process_time()\nwhile time.process_time() - t < 0.3: pass"
        status, _, seconds, cpu = module.run([sys.executable, "-c", spin], Path("."), None)
        self.assertEqual(status, "exit 0")
        self.assertGreaterEqual(cpu, 0.25)
        status, _, seconds, cpu = module.run([sys.executable, "-c", "import time; time.sleep(0.5)"], Path("."), None)
        self.assertEqual((status, seconds >= 0.5), ("exit 0", True))
        self.assertLess(cpu, 0.3)

    def test_a_command_over_its_timeout_is_killed_and_one_that_finishes_is_not_called_timed_out(self):
        module = load("run_tests", SCRIPT)
        child = "subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(30)'])"
        program = f"import subprocess, sys, time; {child}; time.sleep(30)"
        status, _, seconds, _ = module.run([sys.executable, "-c", program], Path("."), 0.3)
        self.assertEqual(status, "timed out")
        self.assertLess(seconds, 5)
        status, output, _, _ = module.run([sys.executable, "-c", "print('done')"], Path("."), 30)
        self.assertEqual((status, output.strip()), ("exit 0", "done"))


if __name__ == "__main__":
    unittest.main()
