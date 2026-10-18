#!/usr/bin/env python3
"""The release suite of the Taxman crates on one machine, with every CPU busy.  Standard library only.

`cargo test` runs the test binaries one after another and the tests of a binary on a few threads, so a long test in one
binary leaves most CPUs of a large machine idle.  This builds all the test executables of every crate at once (`cargo
test --no-run`), lists the tests of each (`--list`; `#[ignore]`d ones are left out, as `cargo test` leaves them out)
and runs every test as a process of its own (`--exact NAME`), JOBS at a time, the longest first, over all the binaries
together.  A failing test is collected, not a reason to stop (like `--no-fail-fast`); the doc tests run as `cargo test
--doc`.

usage: run_tests.py [options] LABEL=DIR [LABEL=DIR ...]      (DIR holds a Cargo.toml; the tests are named LABEL/...)

  --release        test the release profile (`cargo test --release`), as the cache's binaries are built
  --jobs N         tests at once (default: the CPUs this process may use)
  --times FILE     how long each test took in earlier runs (JSON; read if it exists, rewritten after the run): the
                   tests with no entry go first, then the longest.  Without it every test is unknown and they run in
                   the order of their binaries.
  --report FILE    a text report: the totals, the slowest tests, and each failure with its output
  --summary FILE   the counts as JSON (build_cache.py put reads it): crates -> passed, failed, ignored; seconds; jobs
  --timeout S      a test that runs longer than S seconds (default 3600) is killed and counted as failed
  --filter TEXT    only the tests whose name has TEXT in it (the times file keeps the other tests' entries)
  --no-doc         leave out the doc tests
  --nice N         lower the priority of this process and everything it runs

Each test runs in the directory of its crate, as `cargo test` runs it, with the environment of this process.  A test
that fails is run once more, alone, after the others, to say whether it fails only in company: the suite fails either
way.  Exit status: 0 every test passed; 1 a test failed; 2 a build or a listing failed.
"""
import argparse
import collections
import json
import os
import re
import resource
import signal
import subprocess
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor, wait, FIRST_COMPLETED
from pathlib import Path
from typing import Deque, Dict, List, NamedTuple, Optional, Set, Tuple

PASSED = re.compile(r"test result: ok\. 1 passed;")
RESULT_LINE = re.compile(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored")
LISTED = re.compile(r"(.+): test")
PROGRESS_SECONDS = 60
ECHOED_LINES = 40  # of a failure's output, printed when it fails (the report has all of it)


class SuiteError(Exception):
    """The suite cannot be built or listed."""


class Binary(NamedTuple):
    label: str  # the crate: solver, prover
    directory: Path
    kind: str  # lib, bin, test, example
    target: str
    executable: str


class Test(NamedTuple):
    id: str
    binary: Binary
    name: str


class Outcome(NamedTuple):
    id: str
    label: str
    status: str  # passed, failed, timed out
    seconds: float
    output: str  # of a test that did not pass
    passed: int  # tests (a test is one; the doc tests of a crate are as many as it has)
    failed: int


live: Set[subprocess.Popen] = set()
live_lock = threading.Lock()


def say(message: str) -> None:
    print(f"{time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())} tests: {message}", file=sys.stderr, flush=True)


def cpus() -> int:
    return len(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else (os.cpu_count() or 1)


def kill_all() -> None:
    with live_lock:
        for process in list(live):
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except OSError:
                pass


def run(command: List[str], cwd: Path, timeout: Optional[float]) -> Tuple[str, str, float]:
    """Runs a command in a process group of its own; returns (status, output, seconds).  The status is `exit N`, or
    `timed out`."""
    start = time.monotonic()
    env = dict(os.environ, CARGO_MANIFEST_DIR=str(cwd))
    process = subprocess.Popen(command, cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                               stderr=subprocess.STDOUT, start_new_session=True, text=True, errors="replace")
    with live_lock:
        live.add(process)
    try:
        output, _ = process.communicate(timeout=timeout)
        status = f"exit {process.returncode}"
    except subprocess.TimeoutExpired:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except OSError:
            pass
        output, _ = process.communicate()
        status = "timed out"
    finally:
        with live_lock:
            live.discard(process)
    return status, output, time.monotonic() - start


# --- building and listing -------------------------------------------------------------------------------------------
def build(label: str, directory: Path, release: bool) -> List[Binary]:
    """The test executables of a crate: `cargo test --no-run` builds them all and names them."""
    command = ["cargo", "test", *(["--release"] if release else []), "--no-run", "--message-format=json"]
    say(f"building the tests of {label}: {' '.join(command)}")
    process = subprocess.Popen(command, cwd=directory, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                               errors="replace")
    tail: Deque[str] = collections.deque(maxlen=60)  # cargo's messages go to the log as they come; the last are kept

    def pump() -> None:
        for line in process.stderr:
            sys.stderr.write(f"[{label}] {line}")
            tail.append(line)

    pumping = threading.Thread(target=pump)
    pumping.start()
    messages = process.stdout.read()
    process.wait()
    pumping.join()
    if process.returncode != 0:
        raise SuiteError(f"{label}: {' '.join(command)} failed with status {process.returncode}\n{''.join(tail)}")
    found: Dict[str, Binary] = {}
    for line in messages.splitlines():
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if message.get("reason") == "compiler-artifact" and message.get("profile", {}).get("test") \
                and message.get("executable"):
            target = message["target"]
            found[message["executable"]] = Binary(label, directory, target["kind"][0], target["name"],
                                                  message["executable"])
    return list(found.values())


def list_tests(binary: Binary) -> Tuple[List[Test], int]:
    """The tests of a test executable that `cargo test` would run, and the number it would leave out as ignored."""
    def names(*options: str) -> List[str]:
        status, output, _ = run([binary.executable, "--list", *options], binary.directory, 300)
        if status != "exit 0":
            raise SuiteError(f"{binary.executable} --list {' '.join(options)}: {status}\n{output}")
        return [m.group(1) for m in map(LISTED.fullmatch, output.splitlines()) if m]

    ignored = set(names("--ignored"))
    prefix = f"{binary.label}/{binary.kind}:{binary.target}/"
    return [Test(prefix + name, binary, name) for name in names() if name not in ignored], len(ignored)


# --- running ----------------------------------------------------------------------------------------------------------
def run_test(test: Test, timeout: float) -> Outcome:
    status, output, seconds = run([test.binary.executable, "--exact", test.name, "--test-threads=1"],
                                  test.binary.directory, timeout)
    # `--exact` with a name that matches nothing runs no test and succeeds: a test that did not run did not pass.
    if status == "exit 0" and PASSED.search(output):
        return Outcome(test.id, test.binary.label, "passed", seconds, "", 1, 0)
    return Outcome(test.id, test.binary.label, "timed out" if status == "timed out" else "failed", seconds,
                   f"[{status}]\n{output}", 0, 1)


def run_doc_tests(label: str, directory: Path, release: bool, jobs: int) -> Outcome:
    command = ["cargo", "test", *(["--release"] if release else []), "--doc", "--no-fail-fast", "--",
               f"--test-threads={jobs}"]
    status, output, seconds = run(command, directory, None)
    passed = failed = 0
    for line in output.splitlines():
        counted = RESULT_LINE.search(line)
        if counted:
            passed += int(counted.group(1))
            failed += int(counted.group(2))
    ok = status == "exit 0" and failed == 0
    return Outcome(f"{label}/doc", label, "passed" if ok else "failed", seconds, "" if ok else f"[{status}]\n{output}",
                   passed, failed if failed or ok else 1)


def schedule(tests: List[Test], times: Dict[str, float]) -> List[Test]:
    """Unknown tests first, in the order they were listed, then the known ones, the longest first."""
    return sorted(tests, key=lambda t: (0, 0.0) if t.id not in times else (1, -times[t.id]))


def run_all(tests: List[Test], docs: List[Tuple[str, Path]], args: argparse.Namespace, times: Dict[str, float]):
    """Runs the tests, `args.jobs` at a time; returns the outcomes."""
    total = len(tests) + len(docs)
    outcomes: List[Outcome] = []
    started = time.monotonic()
    pool = ThreadPoolExecutor(max_workers=args.jobs)
    try:
        pending = {pool.submit(run_doc_tests, label, directory, args.release, args.jobs) for label, directory in docs}
        pending |= {pool.submit(run_test, test, args.timeout) for test in schedule(tests, times)}
        last_progress = time.monotonic()
        while pending:
            done, pending = wait(pending, timeout=PROGRESS_SECONDS, return_when=FIRST_COMPLETED)
            for future in done:
                outcome = future.result()
                outcomes.append(outcome)
                if outcome.status != "passed":
                    say(f"{outcome.status.upper()}: {outcome.id} ({outcome.seconds:.1f} s)")
                    print("\n".join(outcome.output.splitlines()[-ECHOED_LINES:]), file=sys.stderr, flush=True)
            if time.monotonic() - last_progress >= PROGRESS_SECONDS or not pending:
                failed = sum(o.status != "passed" for o in outcomes)
                say(f"{len(outcomes)} of {total} done, {failed} failed, {min(len(pending), args.jobs)} running, "
                    f"{time.monotonic() - started:.0f} s")
                last_progress = time.monotonic()
    finally:
        pool.shutdown(wait=False, cancel_futures=True)
    return outcomes


# --- the report -------------------------------------------------------------------------------------------------------
def write_times(path: Path, outcomes: List[Outcome], listed: List[Test], old: Dict[str, float], filtered: bool) -> None:
    ids = {t.id for t in listed}
    new = dict(old) if filtered else {k: v for k, v in old.items() if k in ids}
    new.update({o.id: round(o.seconds, 2) for o in outcomes if o.status == "passed" and o.id in ids})
    path.write_text(json.dumps({"tests": new}, indent=0, sort_keys=True) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("crates", nargs="+", metavar="LABEL=DIR")
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--jobs", type=int, default=cpus())
    parser.add_argument("--times")
    parser.add_argument("--report")
    parser.add_argument("--summary")
    parser.add_argument("--timeout", type=float, default=3600)
    parser.add_argument("--filter", default="")
    parser.add_argument("--no-doc", action="store_true")
    parser.add_argument("--nice", type=int, default=0)
    args = parser.parse_args()
    if args.nice:
        os.nice(args.nice)
    signal.signal(signal.SIGTERM, lambda *_: (kill_all(), os._exit(143)))
    signal.signal(signal.SIGINT, lambda *_: (kill_all(), os._exit(130)))

    crates = []
    for spec in args.crates:
        label, _, directory = spec.partition("=")
        if not label or not (Path(directory) / "Cargo.toml").is_file():
            parser.error(f"{spec!r}: expected LABEL=DIR with a Cargo.toml in DIR")
        crates.append((label, Path(directory).resolve()))

    started = time.monotonic()
    try:
        with ThreadPoolExecutor(max_workers=len(crates)) as pool:
            binaries = [b for found in pool.map(lambda c: build(c[0], c[1], args.release), crates) for b in found]
        say(f"built {len(binaries)} test executables in {time.monotonic() - started:.0f} s; listing their tests")
        with ThreadPoolExecutor(max_workers=min(16, args.jobs)) as pool:
            listings = list(pool.map(list_tests, binaries))
    except SuiteError as why:
        print(f"{why}", file=sys.stderr)
        if args.report:
            Path(args.report).write_text(f"the suite could not be built or listed:\n{why}\n")
        return 2
    listed = [t for tests, _ in listings for t in tests]
    ignored: Dict[str, int] = {label: 0 for label, _ in crates}
    for binary, (_, count) in zip(binaries, listings):
        ignored[binary.label] += count
    tests = [t for t in listed if args.filter in t.id]
    docs = [] if args.no_doc or args.filter else [c for c in crates if (c[1] / "src" / "lib.rs").is_file()]

    times: Dict[str, float] = {}
    if args.times and Path(args.times).is_file():
        try:
            times = json.loads(Path(args.times).read_text()).get("tests", {})
        except (ValueError, AttributeError):
            say(f"{args.times} is not a times file: ignored")
    known = sum(t.id in times for t in tests)
    say(f"{len(tests)} tests ({known} with a known time), {sum(ignored.values())} ignored; {args.jobs} at a time")

    cpu_before = resource.getrusage(resource.RUSAGE_CHILDREN)
    pass_started = time.monotonic()
    outcomes = run_all(tests, docs, args, times)
    seconds = time.monotonic() - pass_started
    cpu_after = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = (cpu_after.ru_utime + cpu_after.ru_stime) - (cpu_before.ru_utime + cpu_before.ru_stime)

    by_id = {t.id: t for t in tests}
    failed = [o for o in outcomes if o.status != "passed"]
    alone: Dict[str, str] = {}
    if failed:
        say(f"{len(failed)} failed; running up to 20 of them again, alone")
        for outcome in failed[:20]:
            if outcome.id in by_id:
                again = run_test(by_id[outcome.id], args.timeout)
                alone[outcome.id] = "passes alone" if again.status == "passed" else "fails alone too"

    counts = {label: {"passed": 0, "failed": 0, "ignored": ignored[label]} for label, _ in crates}
    for outcome in outcomes:
        counts[outcome.label]["passed"] += outcome.passed
        counts[outcome.label]["failed"] += outcome.failed

    slowest = sorted(outcomes, key=lambda o: -o.seconds)[:8]
    lines = [f"{label}: {c['passed']} passed, {c['failed']} failed, {c['ignored']} ignored"
             for label, c in counts.items()]
    lines.append(f"{seconds:.0f} s for the tests ({time.monotonic() - started:.0f} s with the build), "
                 f"{args.jobs} at a time, {cpu:.0f} CPU seconds")
    lines.append("slowest: " + ", ".join(f"{o.id} {o.seconds:.1f} s" for o in slowest))
    for line in lines:
        say(line)
    if args.report:
        text = "\n".join(lines) + "\n"
        for outcome in failed:
            text += f"\n==== {outcome.status.upper()}: {outcome.id} ({outcome.seconds:.1f} s)"
            text += f"; {alone[outcome.id]}" if outcome.id in alone else ""
            text += f" ====\n{outcome.output}"
        Path(args.report).write_text(text)
    if args.summary:
        Path(args.summary).write_text(json.dumps({"crates": counts, "seconds": seconds, "cpu_seconds": cpu,
                                                  "jobs": args.jobs, "tests": len(tests)}) + "\n")
    if args.times:
        write_times(Path(args.times), outcomes, listed, times, bool(args.filter))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
