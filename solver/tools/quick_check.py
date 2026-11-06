#!/usr/bin/env python3
"""The quick check of the frame solver's crate: the check that every change passes before it is merged
(solver/docs/design/testing.md). Standard library only.

usage: solver/tools/quick_check.py   (from anywhere; it works in solver/frame and solver/gzip, at a low priority)

It runs `cargo fmt --check`, `cargo clippy --all-targets`, the build of the tests, and the tests of a non-release
build, with solver/tools/cloud/run_tests.py (every test a process of its own, all the CPUs busy).  The gzip decoder's
crate (solver/gzip) has the same fmt and clippy checks, and its tests run in the same suite.  Two checks at once
slow each other, in their builds as much as in their tests, so checks take turns: a check takes a lock (LOCK, a file in
the system's temporary directory, one for every checkout) before it builds anything and holds it until it exits, and a
second check waits for it, saying so.  The system frees the lock when the check ends, however it ends.  The limits
below are for a machine with nothing else to do, so the tests start only when it is quiet: if other work uses more than
BUSY_LIMIT of the CPUs when the build has ended, the check says so and waits.  The wait for the lock and the wait for a
quiet machine count together, BUSY_WAIT seconds at most (the checks and the build between them do not count).  It
exits with status 1 if

  * a check fails or a test fails, or
  * the waits reach BUSY_WAIT seconds together (nothing is built, or no test is run), or
  * the tests take more than TOTAL_LIMIT seconds (their time, not the build's), or
  * a test uses more than TEST_LIMIT seconds of CPU.

Work run at a low priority counts as idle, not as busy: it yields its CPUs to the tests, so a machine that is busy only
with such work is quiet enough.  That is work niced (at a nice value above 0), which the system counts in its `nice`
ticks, and on Linux also work at the SCHED_IDLE policy (`chrt -i 0`) at a nice value of 0 or below, which the system
counts as busy: the check adds the CPU time that such a task used during the sample, from the task's own stat file, to
the idle ticks.  The tests still share a core's hyperthread and the caches with it, which the CPU-second limits feel;
testing.md says what to do if they fail for that.

A test is held to its CPU seconds, not to the seconds it takes: with every CPU busy a test takes several times as long
as it does alone, and its CPU seconds do not change.  The limits are here and nowhere else; raising either needs
the project owner's approval.  A test that takes more than about a second is long: solver/docs/design/testing.md says what to do
with it.
"""
import ctypes
import fcntl
import glob
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Callable, Dict, Iterable, List, Mapping, NamedTuple, Optional

TOTAL_TARGET = 10.0  # seconds the tests should take, on a laptop of ten cores
TOTAL_LIMIT = 15.0  # seconds of the tests after which the check fails
TEST_LIMIT = 3.0  # seconds of CPU one test may use in a non-release run

BUSY_LIMIT = 0.25  # the share of the CPUs that other work may use for the machine to be quiet enough for the limits
BUSY_SAMPLE = 2.0  # seconds one sample of the machine's use of its CPUs takes
BUSY_WAIT = 600.0  # seconds the check waits, for the lock and for the machine to be quiet together, before it fails
BUSY_NOTICE = 30.0  # seconds between the messages that say the machine is still busy, or the lock still held
LOCK_POLL = 1.0  # seconds between the tries at the lock while another check holds it

# Tests at once: a half as many again as there are CPUs, since a few tests wait (for a sync, a timer) with a CPU idle.
JOBS = (os.cpu_count() or 1) * 3 // 2

TOOLS = Path(__file__).resolve().parent
LOCK = Path(tempfile.gettempdir()) / "taxman-quick-check.lock"  # held by the check that is running
CRATE = TOOLS.parent / "frame"  # the frame solver's crate: where the checks run
GZIP = TOOLS.parent / "gzip"  # the gzip decoder's crate: its fmt and clippy checks run here, and its tests in the suite
RUN_TESTS = TOOLS / "cloud" / "run_tests.py"
TIMES = CRATE / "target" / "quick-check-times.json"  # how long the tests took last time: the longest start first
REPORT = CRATE / "target" / "quick-check-report.txt"
SUMMARY = CRATE / "target" / "quick-check-summary.json"

POINTER = "A test that takes more than about a second is long: see solver/docs/design/testing.md."


# --- how busy the machine is ------------------------------------------------------------------------------------------
class Ticks(NamedTuple):
    """What the machine's CPUs have spent so far, all together, in ticks of the system's clock.  `sched_idle` is, on
    Linux, what each task at the SCHED_IDLE policy (and nice 0 or below) has used so far, by task id, which `idle` does
    not hold (see NICED_IS_IDLE); on macOS it is empty."""
    idle: int
    total: int
    sched_idle: Mapping[int, int] = {}


class Task(NamedTuple):
    """The fields of a Linux task's stat file that matter here."""
    tid: int
    ticks: int  # the CPU time the task has used so far, user and system, in ticks of the system's clock
    nice: int
    policy: int


# Work at a low priority does not make the machine busy.  Work niced (a long search at nice 10) or at the SCHED_IDLE
# policy gives its CPUs up to a test the moment the test wants one, so the test runs as it would on a machine with
# nothing else to do, except for what it shares with that work (a core's other hyperthread, the caches).  Both systems'
# `nice` ticks are counted as idle, here, in the one place where the ticks are read.  Linux puts a task's time in its
# `nice` ticks only when the task's nice value is above 0, so work at SCHED_IDLE and nice 0 (`chrt -i 0`) is in the user
# and system ticks, as busy as any: what such tasks used over a sample is counted as idle too (`busy_share`).  A task at
# SCHED_IDLE and nice above 0 is already in the `nice` ticks, and is not counted a second time.
NICED_IS_IDLE = True

SCHED_IDLE = 5  # the number of Linux's SCHED_IDLE policy, field 41 of a task's stat file

WRAP = 1 << 32  # macOS counts ticks in 32 bits, and a laptop that stays up wraps them within weeks


def task_in_stat(text: str) -> Task:
    """Linux: a task's stat file (/proc/PID/task/TID/stat) is `tid (name) state ppid ...`.  The name can hold spaces and
    parentheses, so the fields are counted from the last `)`: the state is field 3, the user and system ticks (utime and
    stime) are fields 14 and 15, nice is field 19, and the policy is field 41 (proc(5))."""
    fields = text[text.rindex(")") + 1:].split()

    def field(number: int) -> int:
        return int(fields[number - 3])

    return Task(int(text.split(None, 1)[0]), field(14) + field(15), field(19), field(41))


def sched_idle_ticks(task_stats: Iterable[str]) -> Dict[int, int]:
    """Linux: the ticks so far of the tasks at the SCHED_IDLE policy and nice 0 or below, by task id, from the text of
    the tasks' stat files.  A task above nice 0 is in the `nice` ticks of /proc/stat already (see NICED_IS_IDLE)."""
    tasks = map(task_in_stat, task_stats)
    return {task.tid: task.ticks for task in tasks if task.policy == SCHED_IDLE and task.nice <= 0}


def ticks_in_proc_stat(text: str, task_stats: Iterable[str] = ()) -> Ticks:
    """Linux: the first line of /proc/stat is `cpu user nice system idle iowait irq softirq steal ...`, the ticks of all
    the CPUs together; a CPU that waits for input or output is idle, and so is one that runs niced work (see
    NICED_IS_IDLE).  The `task_stats` are the texts of the tasks' stat files, for the tasks at SCHED_IDLE."""
    fields = [int(field) for field in text.splitlines()[0].split()[1:9]]
    return Ticks(fields[3] + fields[4] + fields[1], sum(fields), sched_idle_ticks(task_stats))


def ticks_of_mach() -> Ticks:
    """macOS: the host's `host_cpu_load_info`, the ticks of user, system, idle and nice time of all the CPUs; nice time
    is idle (see NICED_IS_IDLE)."""
    libc = ctypes.CDLL("/usr/lib/libSystem.dylib")
    libc.mach_host_self.restype = ctypes.c_uint32
    libc.host_statistics.argtypes = [ctypes.c_uint32, ctypes.c_int, ctypes.POINTER(ctypes.c_uint32),
                                     ctypes.POINTER(ctypes.c_uint32)]
    libc.host_statistics.restype = ctypes.c_int
    host_cpu_load_info, user, system, idle, nice = 3, 0, 1, 2, 3
    info, count = (ctypes.c_uint32 * 4)(), ctypes.c_uint32(4)
    if libc.host_statistics(libc.mach_host_self(), host_cpu_load_info, info, ctypes.byref(count)) != 0:
        raise OSError("host_statistics failed")
    return Ticks((info[idle] + info[nice]) % WRAP, (info[user] + info[system] + info[idle] + info[nice]) % WRAP)


def read_task_stats() -> List[str]:
    """Linux: the text of the stat file of every task (every thread of every process) now.  A task that ends while the
    files are read, or a file that cannot be read, is left out.  A task's name can be any bytes, so they are decoded
    leniently."""
    texts = []
    for path in glob.glob("/proc/[0-9]*/task/[0-9]*/stat"):
        try:
            texts.append(Path(path).read_text(errors="replace"))
        except OSError:
            continue
    return texts


def read_ticks() -> Optional[Ticks]:
    """The ticks of the machine's CPUs so far, or None where this system does not say."""
    try:
        if sys.platform == "darwin":
            return ticks_of_mach()
        return ticks_in_proc_stat(Path("/proc/stat").read_text(), read_task_stats())
    except (OSError, ValueError, IndexError, AttributeError):
        return None


def sched_idle_growth(before: Ticks, after: Ticks) -> int:
    """The ticks the tasks at SCHED_IDLE used between two readings: those of the tasks in both, since a task that began
    or ended between them has no ticks to compare."""
    both = before.sched_idle.keys() & after.sched_idle.keys()
    return sum(after.sched_idle[tid] - before.sched_idle[tid] for tid in both)


def busy_share(before: Ticks, after: Ticks) -> Optional[float]:
    """The share of the CPUs' time between two readings that was not idle (0 to 1), or None if no ticks went by.  The
    ticks that tasks at SCHED_IDLE used count as idle (see NICED_IS_IDLE)."""
    total = (after.total - before.total) % WRAP
    if total == 0:
        return None
    idle = (after.idle - before.idle) % WRAP + sched_idle_growth(before, after)
    return min(1.0, max(0.0, 1.0 - idle / total))


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
                     who: Callable[[], str] = lambda: "", waited_before: float = 0.0) -> Optional[str]:
    """Takes samples of the machine's use of its CPUs until one is BUSY_LIMIT or less, saying why it waits.  Returns
    None once the machine is quiet (or if it cannot be measured on this system: there is nothing to wait for), and the
    message of the failure if it is still busy when the waiting has reached BUSY_WAIT seconds, the `waited_before` of
    the wait for the lock included."""
    started = clock() - waited_before
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


# --- taking turns -----------------------------------------------------------------------------------------------------
def open_lock(path: Path = LOCK) -> int:
    """Opens (and makes, if it is not there) the lock's file, for reading only so that another user's file can be
    locked too.  The descriptor is not inherited by the processes this check starts (Python's default), so none of them
    can outlive it and keep the lock."""
    return os.open(path, os.O_RDONLY | os.O_CREAT, 0o666)


def take_lock(fd: int) -> bool:
    """Tries once to take the exclusive lock on the open file `fd`: whether it is ours now.  A flock is held by the open
    file, and the system frees it when the descriptor is closed or the process ends, a crash included."""
    try:
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        return False
    return True


def wait_for_turn(take: Callable[[], bool], clock: Callable[[], float], sleep: Callable[[float], None],
                  say: Callable[[str], None]) -> Optional[str]:
    """Tries `take` every LOCK_POLL seconds until the lock is ours, saying that another check holds it.  Returns None
    once it is ours, and the message of the failure if it is still held after BUSY_WAIT seconds."""
    started = clock()
    noticed: Optional[float] = None
    while not take():
        waited = clock() - started
        if waited >= BUSY_WAIT:
            return (f"another quick check held the lock for the {BUSY_WAIT / 60:.0f} minutes this check waited, and "
                    f"the limits are for one check at a time, so nothing was built or run; wait until it ends, and "
                    f"run the check again")
        if noticed is None or clock() - noticed >= BUSY_NOTICE:
            say(f"another quick check is running; this one waits for it to end before it builds anything, so that the "
                f"two do not slow each other (waited {waited:.0f} s, gives up at {BUSY_WAIT:.0f} s)")
            noticed = clock()
        sleep(LOCK_POLL)
    if noticed is not None:
        say(f"the other quick check has ended, after {clock() - started:.0f} s")
    return None


# --- the checks ------------------------------------------------------------------------------------------------------
def problems(summary: dict) -> List[str]:
    """What is over the limits in the summary that run_tests.py wrote (`--summary`), one message each."""
    found = []
    seconds = summary["seconds"]
    if seconds > TOTAL_LIMIT:
        slowest = ", ".join(f"{t['id']} ({t['cpu']:.1f} s of CPU)" for t in summary["costs"][:5])
        found.append(
            f"the tests took {seconds:.1f} s; the limit is {TOTAL_LIMIT:.0f} s (the target is {TOTAL_TARGET:.0f} s).  "
            f"The tests that used the most CPU: {slowest}"
        )
    for test in summary["costs"]:  # every test, not only the costliest few
        if test["cpu"] > TEST_LIMIT:
            found.append(
                f"{test['id']} used {test['cpu']:.1f} s of CPU ({test['seconds']:.1f} s on the clock); "
                f"the limit is {TEST_LIMIT:.0f} s"
            )
    return found


def say(message: str) -> None:
    print(f"quick check: {message}", file=sys.stderr, flush=True)


def step(name: str, command: List[str], crate: Path = CRATE) -> bool:
    say(f"{name}: {' '.join(command)}")
    return subprocess.run(command, cwd=crate).returncode == 0


def run_checks(waited_for_lock: float) -> List[str]:
    """Runs fmt, clippy and the build, waits for a quiet machine (the seconds already waited for the lock count against
    BUSY_WAIT), then runs the tests.  Returns what failed, one message each.  The gzip decoder's crate has its fmt and
    clippy here, and its tests build and run with the frame solver's."""
    failed = []
    for name, command, crate in [("fmt", ["cargo", "fmt", "--check"], CRATE),
                                 ("clippy", ["cargo", "clippy", "--all-targets"], CRATE),
                                 ("build", ["cargo", "test", "--no-run"], CRATE),
                                 ("gzip fmt", ["cargo", "fmt", "--check"], GZIP),
                                 ("gzip clippy", ["cargo", "clippy", "--all-targets"], GZIP)]:
        if not step(name, command, crate):
            failed.append(f"{name} failed")
    SUMMARY.unlink(missing_ok=True)
    if "build failed" not in failed:
        busy = wait_until_quiet(sample_busy_share, time.monotonic, say, who_is_busy, waited_for_lock)
        if busy:
            failed.append(busy)
        else:
            TIMES.parent.mkdir(exist_ok=True)
            tests = [sys.executable, "-I", str(RUN_TESTS), "--jobs", str(JOBS), "--times", str(TIMES), "--report",
                     str(REPORT), "--summary", str(SUMMARY), f"solver={CRATE}", f"gzip={GZIP}"]
            if not step("tests", tests):
                failed.append(f"tests failed (the report is {REPORT})")
    return failed


def main() -> int:
    os.nice(10)
    began = time.monotonic()
    lock = open_lock()  # held until this process ends, however it ends: the whole check runs while it is
    busy = wait_for_turn(lambda: take_lock(lock), time.monotonic, time.sleep, say)
    if busy:  # nothing was built or run: a summary in target/ is an earlier check's
        failed, over = [busy], []
    else:
        failed = run_checks(time.monotonic() - began)
        over = problems(json.loads(SUMMARY.read_text())) if SUMMARY.is_file() else []
    if failed or over:
        print("\nquick check FAILED", file=sys.stderr)
        for message in failed + over:
            print(f"  {message}", file=sys.stderr)
        if over:
            print(f"  {POINTER}\n  The limits are in solver/tools/quick_check.py; raising one needs the project owner's approval.",
                  file=sys.stderr)
        return 1
    seconds = json.loads(SUMMARY.read_text())["seconds"]
    note = "" if seconds <= TOTAL_TARGET else f" (the target is {TOTAL_TARGET:.0f} s)"
    say(f"passed: the tests took {seconds:.1f} s of {TOTAL_LIMIT:.0f} s{note}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
