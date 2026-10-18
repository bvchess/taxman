#!/usr/bin/env python3
"""The quick check of the Rust crate: the check that every change passes before it is merged (DESIGN.md, section 9).
Standard library only.

usage: tools/quick_check.py          (from anywhere; it works in rust/ and runs at a low priority)

It runs `cargo fmt --check`, `cargo clippy --all-targets` and the tests of a non-release build, with
tools/cloud/run_tests.py (every test a process of its own, all the CPUs busy), and exits with status 1 if

  * a check fails or a test fails, or
  * the tests take more than TOTAL_LIMIT seconds (their time, not the build's), or
  * a test uses more than TEST_LIMIT seconds of CPU.

A test is held to its CPU seconds, not to the seconds it takes: with every CPU busy a test takes several times as long
as it does alone, and its CPU seconds do not change.  The limits are here and nowhere else; raising either needs
Brian's approval.  A test that takes more than about a second is long: DESIGN.md, section 9, says what to do with it.
"""
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import List

TOTAL_TARGET = 10.0  # seconds the tests should take, on a laptop of ten cores
TOTAL_LIMIT = 15.0  # seconds of the tests after which the check fails
TEST_LIMIT = 2.0  # seconds of CPU one test may use in a non-release run

# Tests at once: a half as many again as there are CPUs, since a few tests wait (for a sync, a timer) with a CPU idle.
JOBS = (os.cpu_count() or 1) * 3 // 2

CRATE = Path(__file__).resolve().parent.parent
RUN_TESTS = CRATE / "tools" / "cloud" / "run_tests.py"
TIMES = CRATE / "target" / "quick-check-times.json"  # how long the tests took last time: the longest start first
REPORT = CRATE / "target" / "quick-check-report.txt"
SUMMARY = CRATE / "target" / "quick-check-summary.json"

POINTER = "A test that takes more than about a second is long: see rust/DESIGN.md, section 9, 'Testing'."


def problems(summary: dict) -> List[str]:
    """What is over the limits in the summary that run_tests.py wrote (`--summary`), one message each."""
    found = []
    seconds = summary["seconds"]
    if seconds > TOTAL_LIMIT:
        slowest = ", ".join(f"{t['id']} ({t['cpu']:.1f} s of CPU)" for t in summary["costliest"][:5])
        found.append(
            f"the tests took {seconds:.1f} s; the limit is {TOTAL_LIMIT:.0f} s (the target is {TOTAL_TARGET:.0f} s).  "
            f"The tests that used the most CPU: {slowest}"
        )
    for test in summary["costliest"]:
        if test["cpu"] > TEST_LIMIT:
            found.append(
                f"{test['id']} used {test['cpu']:.1f} s of CPU ({test['seconds']:.1f} s on the clock); "
                f"the limit is {TEST_LIMIT:.0f} s"
            )
    return found


def step(name: str, command: List[str]) -> bool:
    print(f"quick check: {name}: {' '.join(command)}", file=sys.stderr, flush=True)
    return subprocess.run(command, cwd=CRATE).returncode == 0


def main() -> int:
    os.nice(10)
    failed = []
    for name, command in [("fmt", ["cargo", "fmt", "--check"]), ("clippy", ["cargo", "clippy", "--all-targets"])]:
        if not step(name, command):
            failed.append(name)
    SUMMARY.unlink(missing_ok=True)
    TIMES.parent.mkdir(exist_ok=True)
    tests = [sys.executable, "-I", str(RUN_TESTS), "--jobs", str(JOBS), "--times", str(TIMES), "--report", str(REPORT),
             "--summary", str(SUMMARY), f"solver={CRATE}"]
    if not step("tests", tests):
        failed.append(f"tests (the report is {REPORT})")
    over = problems(json.loads(SUMMARY.read_text())) if SUMMARY.is_file() else []
    if failed or over:
        print("\nquick check FAILED", file=sys.stderr)
        for name in failed:
            print(f"  {name} failed", file=sys.stderr)
        for message in over:
            print(f"  {message}", file=sys.stderr)
        if over:
            print(f"  {POINTER}\n  The limits are in rust/tools/quick_check.py; raising one needs Brian's approval.",
                  file=sys.stderr)
        return 1
    seconds = json.loads(SUMMARY.read_text())["seconds"]
    note = "" if seconds <= TOTAL_TARGET else f" (the target is {TOTAL_TARGET:.0f} s)"
    print(f"quick check passed: the tests took {seconds:.1f} s of {TOTAL_LIMIT:.0f} s{note}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
