#!/usr/bin/env python3
"""The quick check of the frame solver's crate: the check that every change passes before it is merged (DESIGN.md,
section 9). Standard library only.

usage: solver/tools/quick_check.py   (from anywhere; it works in solver/frame and runs at a low priority)

It runs `cargo fmt --check`, `cargo clippy --all-targets`, the build of the tests, and the tests of a non-release
build, with solver/tools/cloud/run_tests.py (every test a process of its own, all the CPUs busy).  The limits below are
for a machine with nothing else to do, so the tests start only when it is quiet: if other work uses more than
BUSY_LIMIT of the CPUs when the build has ended, the check says so and waits, for BUSY_WAIT seconds at most.  It exits
with status 1 if

  * a check fails or a test fails, or
  * the machine stays busy for BUSY_WAIT seconds (the tests are not run), or
  * the tests take more than TOTAL_LIMIT seconds (their time, not the build's), or
  * a test uses more than TEST_LIMIT seconds of CPU.

A test is held to its CPU seconds, not to the seconds it takes: with every CPU busy a test takes several times as long
as it does alone, and its CPU seconds do not change.  The limits are here and nowhere else; raising either needs
Brian's approval.  A test that takes more than about a second is long: DESIGN.md, section 9, says what to do with it.
"""
import ctypes
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Callable, Dict, List, NamedTuple, Optional

TOTAL_TARGET = 10.0  # seconds the tests should take, on a laptop of ten cores
TOTAL_LIMIT = 15.0  # seconds of the tests after which the check fails
TEST_LIMIT = 2.0  # seconds of CPU one test may use in a non-release run

BUSY_LIMIT = 0.25  # the share of the CPUs that other work may use for the machine to be quiet enough for the limits
BUSY_SAMPLE = 2.0  # seconds one sample of the machine's use of its CPUs takes
BUSY_WAIT = 600.0  # seconds the check waits for the machine to be quiet before it fails
BUSY_NOTICE = 30.0  # seconds between the messages that say the machine is still busy

# Tests at once: a half as many again as there are CPUs, since a few tests wait (for a sync, a timer) with a CPU idle.
JOBS = (os.cpu_count() or 1) * 3 // 2

TOOLS = Path(__file__).resolve().parent
CRATE = TOOLS.parent / "frame"  # the frame solver's crate: where the checks run
RUN_TESTS = TOOLS / "cloud" / "run_tests.py"
TIMES = CRATE / "target" / "quick-check-times.json"  # how long the tests took last time: the longest start first
REPORT = CRATE / "target" / "quick-check-report.txt"
SUMMARY = CRATE / "target" / "quick-check-summary.json"

POINTER = "A test that takes more than about a second is long: see solver/docs/DESIGN.md, section 9, 'Testing'."


# --- how busy the machine is ------------------------------------------------------------------------------------------
class Ticks(NamedTuple):
    """What the machine's CPUs have spent so far, all together, in ticks of the system's clock."""
    idle: int
    total: int


WRAP = 1 << 32  # macOS counts ticks in 32 bits, and a laptop that stays up wraps them within weeks


def ticks_in_proc_stat(text: str) -> Ticks:
    """Linux: the first line of /proc/stat is `cpu user nice system idle iowait irq softirq steal ...`, the ticks of all
    the CPUs together; a CPU that waits for input or output is idle."""
    fields = [int(field) for field in text.splitlines()[0].split()[1:9]]
    return Ticks(fields[3] + fields[4], sum(fields))


def ticks_of_mach() -> Ticks:
    """macOS: the host's `host_cpu_load_info`, the ticks of user, system, idle and nice time of all the CPUs."""
    libc = ctypes.CDLL("/usr/lib/libSystem.dylib")
    libc.mach_host_self.restype = ctypes.c_uint32
    libc.host_statistics.argtypes = [ctypes.c_uint32, ctypes.c_int, ctypes.POINTER(ctypes.c_uint32),
                                     ctypes.POINTER(ctypes.c_uint32)]
    libc.host_statistics.restype = ctypes.c_int
    host_cpu_load_info, user, system, idle, nice = 3, 0, 1, 2, 3
    info, count = (ctypes.c_uint32 * 4)(), ctypes.c_uint32(4)
    if libc.host_statistics(libc.mach_host_self(), host_cpu_load_info, info, ctypes.byref(count)) != 0:
        raise OSError("host_statistics failed")
    return Ticks(info[idle], (info[user] + info[system] + info[idle] + info[nice]) % WRAP)


def read_ticks() -> Optional[Ticks]:
    """The ticks of the machine's CPUs so far, or None where this system does not say."""
    try:
        return ticks_of_mach() if sys.platform == "darwin" else ticks_in_proc_stat(Path("/proc/stat").read_text())
    except (OSError, ValueError, IndexError, AttributeError):
        return None


def busy_share(before: Ticks, after: Ticks) -> Optional[float]:
    """The share of the CPUs' time between two readings that was not idle (0 to 1), or None if no ticks went by."""
    total = (after.total - before.total) % WRAP
    if total == 0:
        return None
    return min(1.0, max(0.0, 1.0 - ((after.idle - before.idle) % WRAP) / total))


def sample_busy_share() -> Optional[float]:
    """The share of the CPUs that everything on the machine used in a sample of BUSY_SAMPLE seconds.  The sample counts
    every process, ours too, so it is taken when none of ours is running, after the build: the build's own load is then
    out of it at once, as it is not out of the 1-minute load average."""
    before = read_ticks()
    if before is None:
        return None
    time.sleep(BUSY_SAMPLE)
    after = read_ticks()
    return None if after is None else busy_share(before, after)


def busiest(ps_output: str, own_pid: int, count: int = 3) -> str:
    """The processes that use the CPUs most, not ours, from `ps -Ao pid=,ppid=,pcpu=,comm=`: 'name (share%), ...'.  The
    share is the system's recent average for the process, a hint of who is at work."""
    rows: Dict[int, List[str]] = {}
    for line in ps_output.splitlines():
        fields = line.split(None, 3)
        if len(fields) == 4:
            rows[int(fields[0])] = fields[1:]

    def ours(pid: int) -> bool:
        seen = set()
        while pid in rows and pid != own_pid and pid not in seen:
            seen.add(pid)
            pid = int(rows[pid][0])
        return pid == own_pid

    mine = {pid for pid in rows if ours(pid)}
    others = sorted(((float(row[1]), row[2].rsplit("/", 1)[-1]) for pid, row in rows.items() if pid not in mine),
                    reverse=True)
    return ", ".join(f"{name} ({cpu:.0f}%)" for cpu, name in others[:count] if cpu >= 5)


def who_is_busy() -> str:
    """[`busiest`] of this system's processes now; empty where `ps` does not say."""
    try:
        run = subprocess.run(["ps", "-Ao", "pid=,ppid=,pcpu=,comm="], capture_output=True, text=True, timeout=10)
        return busiest(run.stdout, os.getpid()) if run.returncode == 0 else ""
    except (OSError, ValueError, subprocess.SubprocessError):
        return ""


def wait_until_quiet(sample: Callable[[], Optional[float]], clock: Callable[[], float], say: Callable[[str], None],
                     who: Callable[[], str] = lambda: "") -> Optional[str]:
    """Takes samples of the machine's use of its CPUs until one is BUSY_LIMIT or less, saying why it waits.  Returns
    None once the machine is quiet (or if it cannot be measured on this system: there is nothing to wait for), and the
    message of the failure if it is still busy after BUSY_WAIT seconds."""
    started = clock()
    noticed: Optional[float] = None
    while True:
        share = sample()
        waited = clock() - started
        if share is None:
            say("cannot measure how busy the machine is on this system; the tests start without waiting for it")
            return None
        if share <= BUSY_LIMIT:
            after = f", after {waited:.0f} s" if noticed is not None else ""
            say(f"the machine is quiet: {share:.1%} of the CPUs in use{after}")
            return None
        giving_up = waited >= BUSY_WAIT
        if not giving_up and noticed is not None and clock() - noticed < BUSY_NOTICE:
            continue
        culprits = who()
        busiest_now = f" (the busiest: {culprits})" if culprits else ""
        if giving_up:
            return (f"machine busy: other work used {share:.1%} of the CPUs{busiest_now} for the {BUSY_WAIT / 60:.0f} "
                    f"minutes the check waited, and the limits are for {BUSY_LIMIT:.0%} or less, so the tests were "
                    f"not run; wait until the other work ends, or stop it, and run the check again")
        say(f"machine busy: other work uses {share:.1%} of the CPUs{busiest_now}; the limits are for {BUSY_LIMIT:.0%} "
            f"or less, so the tests wait for it to be quiet (waited {waited:.0f} s, gives up at {BUSY_WAIT:.0f} s)")
        noticed = clock()


# --- the checks ------------------------------------------------------------------------------------------------------
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


def say(message: str) -> None:
    print(f"quick check: {message}", file=sys.stderr, flush=True)


def step(name: str, command: List[str]) -> bool:
    say(f"{name}: {' '.join(command)}")
    return subprocess.run(command, cwd=CRATE).returncode == 0


def main() -> int:
    os.nice(10)
    failed = []
    for name, command in [("fmt", ["cargo", "fmt", "--check"]), ("clippy", ["cargo", "clippy", "--all-targets"]),
                          ("build", ["cargo", "test", "--no-run"])]:
        if not step(name, command):
            failed.append(f"{name} failed")
    SUMMARY.unlink(missing_ok=True)
    if "build failed" not in failed:
        busy = wait_until_quiet(sample_busy_share, time.monotonic, say, who_is_busy)
        if busy:
            failed.append(busy)
        else:
            TIMES.parent.mkdir(exist_ok=True)
            tests = [sys.executable, "-I", str(RUN_TESTS), "--jobs", str(JOBS), "--times", str(TIMES), "--report",
                     str(REPORT), "--summary", str(SUMMARY), f"solver={CRATE}"]
            if not step("tests", tests):
                failed.append(f"tests failed (the report is {REPORT})")
    over = problems(json.loads(SUMMARY.read_text())) if SUMMARY.is_file() else []
    if failed or over:
        print("\nquick check FAILED", file=sys.stderr)
        for message in failed + over:
            print(f"  {message}", file=sys.stderr)
        if over:
            print(f"  {POINTER}\n  The limits are in solver/tools/quick_check.py; raising one needs Brian's approval.",
                  file=sys.stderr)
        return 1
    seconds = json.loads(SUMMARY.read_text())["seconds"]
    note = "" if seconds <= TOTAL_TARGET else f" (the target is {TOTAL_TARGET:.0f} s)"
    say(f"passed: the tests took {seconds:.1f} s of {TOTAL_LIMIT:.0f} s{note}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
