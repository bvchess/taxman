#!/usr/bin/env python3
"""Unit tests of quick_check.py's judgment of a run (the summary that run_tests.py writes) and of its wait for a quiet
machine (the CPU ticks, the tasks at SCHED_IDLE whose ticks count as idle, the share the ticks say is busy, and the
decision to wait, go on or give up) and of its lock, which makes two checks take turns.

usage: python3 -I test_quick_check.py      (or python3 -I -m unittest test_quick_check, from this directory)
"""
import os
import signal
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import List, Optional

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))  # python -I puts neither the script's directory nor the current one on the path

import quick_check  # noqa: E402


def summary(seconds, *tests):
    """What run_tests.py writes: the seconds of the tests and every test's id, seconds and cpu, the costliest first."""
    costs = [{"id": name, "seconds": wall, "cpu": cpu} for name, wall, cpu in tests]
    return {"seconds": seconds, "costs": sorted(costs, key=lambda t: -t["cpu"])}


class Limits(unittest.TestCase):
    def test_a_run_inside_the_limits_has_no_problem(self):
        run = summary(quick_check.TOTAL_LIMIT, ("a", 5.0, quick_check.TEST_LIMIT))
        self.assertEqual(quick_check.problems(run), [])

    def test_tests_over_the_total_limit_are_a_problem_naming_the_costliest(self):
        found = quick_check.problems(summary(quick_check.TOTAL_LIMIT + 1, ("a", 1.0, 1.5), ("b", 1.0, 0.5)))
        self.assertEqual(len(found), 1)
        self.assertIn("the tests took", found[0])
        self.assertIn("a (1.5 s of CPU)", found[0])

    def test_a_test_over_the_cpu_limit_is_named_whatever_the_total(self):
        found = quick_check.problems(summary(1.0, ("slow", 9.0, quick_check.TEST_LIMIT + 0.1), ("fine", 9.0, 1.0)))
        self.assertEqual(len(found), 1)
        self.assertIn("slow used", found[0])

    def test_every_test_over_the_cpu_limit_is_named_however_many_there_are(self):
        over = [(f"over{i}", 9.0, quick_check.TEST_LIMIT + 0.1 + i / 10) for i in range(12)]
        under = [(f"under{i}", 1.0, quick_check.TEST_LIMIT - 0.1) for i in range(20)]
        found = quick_check.problems(summary(1.0, *over, *under))
        self.assertEqual(len(found), len(over))
        for name, _, _ in over:
            self.assertEqual(sum(f"{name} used" in message for message in found), 1, name)

    def test_a_test_that_takes_long_on_a_busy_machine_but_uses_little_cpu_is_not_a_problem(self):
        self.assertEqual(quick_check.problems(summary(1.0, ("waits", 30.0, 0.4))), [])

    def test_the_target_is_below_the_limit(self):
        self.assertLess(quick_check.TOTAL_TARGET, quick_check.TOTAL_LIMIT)


class Ticks(unittest.TestCase):
    def test_linux_reads_the_cpu_line_of_proc_stat_and_counts_waiting_for_input_and_niced_work_as_idle(self):
        text = "cpu  100 5 50 800 20 0 5 0 0 0\ncpu0 50 2 25 400 10 0 2 0 0 0\nintr 12345\n"
        self.assertEqual(quick_check.ticks_in_proc_stat(text), quick_check.Ticks(idle=825, total=980))

    def test_a_machine_busy_with_niced_work_only_is_quiet(self):
        before = quick_check.ticks_in_proc_stat("cpu  500 100000 400 1000 0 0 0 0 0 0\n")
        after = quick_check.ticks_in_proc_stat("cpu  550 200000 450 1100 0 0 0 0 0 0\n")
        self.assertAlmostEqual(quick_check.busy_share(before, after), 100 / 100200)

    def test_the_share_is_what_the_ticks_between_two_readings_say_was_not_idle(self):
        before = quick_check.Ticks(idle=1000, total=2000)
        self.assertEqual(quick_check.busy_share(before, quick_check.Ticks(idle=1300, total=2400)), 0.25)
        self.assertEqual(quick_check.busy_share(before, quick_check.Ticks(idle=1400, total=2400)), 0.0)
        self.assertEqual(quick_check.busy_share(before, quick_check.Ticks(idle=1000, total=2400)), 1.0)

    def test_the_share_survives_the_wrap_of_a_32_bit_counter(self):
        wrap = quick_check.WRAP
        before = quick_check.Ticks(idle=wrap - 100, total=wrap - 200)
        after = quick_check.Ticks(idle=100, total=300)  # 200 idle of 500 ticks went by
        self.assertAlmostEqual(quick_check.busy_share(before, after), 0.6)

    def test_no_ticks_between_two_readings_say_nothing(self):
        same = quick_check.Ticks(idle=5, total=10)
        self.assertIsNone(quick_check.busy_share(same, same))

    def test_this_system_gives_ticks_that_only_grow_or_says_it_cannot(self):
        first = quick_check.read_ticks()
        if first is None:
            self.skipTest("this system does not say how busy its CPUs are")
        second = quick_check.read_ticks()
        self.assertGreaterEqual((second.total - first.total) % quick_check.WRAP, 0)
        self.assertLess((second.total - first.total) % quick_check.WRAP, 1 << 20, "a counter that jumped")


def proc_stat(user=0, nice=0, system=0, idle=0) -> str:
    """The cpu line of /proc/stat: user nice system idle iowait irq softirq steal guest guest_nice."""
    return f"cpu  {user} {nice} {system} {idle} 0 0 0 0 0 0\n"


def task_stat(tid: int, name: str = "exact", ticks=(0, 0), nice: int = 0, policy: int = 0) -> str:
    """A task's stat file as Linux writes it, 52 fields: the task id, the name in parentheses, and numbers, of which
    only the state (field 3), utime and stime (14 and 15), nice (19) and policy (41) are set."""
    fields = {3: "R", 14: ticks[0], 15: ticks[1], 19: nice, 41: policy}
    return " ".join([str(tid), f"({name})"] + [str(fields.get(number, 0)) for number in range(3, 53)])


# A thread of the prover on batmobile, which runs at SCHED_IDLE (policy 5) and nice 0.
PROVER_THREAD = (
    "1496520 (exact) R 1383891 1496517 1383889 34817 1383889 4194368 532575 0 7 0 179267 117 0 0 20 0 89 0 135949331 "
    "29550268416 689966 18446744073709551615 94796093706608 94796094249440 140729367706224 0 0 0 0 3149824 17474 0 0 0 "
    "-1 12 0 5 0 0 0 94796094282520 94796094285168 94796304818176 140729367711203 140729367723043 140729367723043 "
    "140729367723957 0\n"
)


class SchedIdleTasks(unittest.TestCase):
    """Linux counts the time of a task at SCHED_IDLE and nice 0 as busy; the check counts it as idle."""

    def test_a_stat_file_is_read_by_field_number_from_the_last_parenthesis(self):
        self.assertEqual(quick_check.task_in_stat(PROVER_THREAD),
                         quick_check.Task(tid=1496520, ticks=179267 + 117, nice=0, policy=quick_check.SCHED_IDLE))

    def test_a_name_with_spaces_and_parentheses_does_not_shift_the_fields(self):
        text = task_stat(77, name="a b) (c (d) e", ticks=(300, 40), nice=-5, policy=3)
        self.assertEqual(quick_check.task_in_stat(text), quick_check.Task(tid=77, ticks=340, nice=-5, policy=3))

    def test_only_tasks_at_sched_idle_and_nice_0_or_below_are_taken_by_task_id(self):
        stats = [task_stat(1, ticks=(10, 1), policy=quick_check.SCHED_IDLE),
                 task_stat(2, ticks=(20, 2), nice=-5, policy=quick_check.SCHED_IDLE),
                 task_stat(3, ticks=(30, 3), nice=10, policy=quick_check.SCHED_IDLE),  # in the `nice` ticks already
                 task_stat(4, ticks=(40, 4), policy=0),
                 task_stat(5, ticks=(50, 5), nice=10, policy=0),
                 task_stat(6, ticks=(60, 6), policy=3)]  # SCHED_BATCH
        self.assertEqual(quick_check.sched_idle_ticks(stats), {1: 11, 2: 22})

    def test_a_machine_busy_with_sched_idle_tasks_at_nice_0_only_is_quiet(self):
        idle = quick_check.SCHED_IDLE
        before = quick_check.ticks_in_proc_stat(
            proc_stat(user=500, system=400, idle=1000),
            [task_stat(1, ticks=(100, 10), policy=idle), task_stat(2, ticks=(50, 5), policy=idle)])
        after = quick_check.ticks_in_proc_stat(
            proc_stat(user=730, system=415, idle=1000),
            [task_stat(1, ticks=(250, 20), policy=idle), task_stat(2, ticks=(130, 10), policy=idle)])
        self.assertEqual(quick_check.busy_share(before, after), 0.0)

    def test_the_machine_is_busy_with_what_other_tasks_use_beside_the_sched_idle_ones(self):
        idle = quick_check.SCHED_IDLE
        before = quick_check.ticks_in_proc_stat(
            proc_stat(user=1000), [task_stat(1, ticks=(100, 0), policy=idle), task_stat(2, ticks=(100, 0))])
        after = quick_check.ticks_in_proc_stat(
            proc_stat(user=1400), [task_stat(1, ticks=(400, 0), policy=idle), task_stat(2, ticks=(200, 0))])
        self.assertEqual(quick_check.busy_share(before, after), 0.25)

    def test_a_niced_sched_idle_task_is_idle_once_through_the_nice_ticks_and_not_counted_again(self):
        idle = quick_check.SCHED_IDLE
        before = quick_check.ticks_in_proc_stat(
            proc_stat(user=1000, nice=0),
            [task_stat(1, ticks=(100, 0), nice=10, policy=idle), task_stat(2, ticks=(100, 0))])
        after = quick_check.ticks_in_proc_stat(
            proc_stat(user=1200, nice=200),  # 200 ticks of the niced task, 200 of the other
            [task_stat(1, ticks=(300, 0), nice=10, policy=idle), task_stat(2, ticks=(300, 0))])
        self.assertEqual(quick_check.busy_share(before, after), 0.5)

    def test_a_task_in_only_one_reading_is_left_out(self):
        idle = quick_check.SCHED_IDLE
        before = quick_check.ticks_in_proc_stat(
            proc_stat(user=1000),
            [task_stat(1, ticks=(100, 0), policy=idle), task_stat(2, ticks=(900, 0), policy=idle)])  # 2 ends
        after = quick_check.ticks_in_proc_stat(
            proc_stat(user=2000),
            [task_stat(1, ticks=(200, 0), policy=idle), task_stat(3, ticks=(5000, 0), policy=idle)])  # 3 begins
        self.assertEqual(quick_check.sched_idle_growth(before, after), 100)
        self.assertEqual(quick_check.busy_share(before, after), 0.9)

    def test_readings_without_tasks_say_what_the_cpu_ticks_say(self):
        before = quick_check.ticks_in_proc_stat(proc_stat(user=100, idle=100))
        after = quick_check.ticks_in_proc_stat(proc_stat(user=200, idle=100))
        self.assertEqual(before.sched_idle, {})
        self.assertEqual(quick_check.busy_share(before, after), 1.0)

    def test_every_task_this_system_lists_is_read(self):
        if not sys.platform.startswith("linux"):
            self.skipTest("only Linux has /proc/PID/task/TID/stat")
        stats = quick_check.read_task_stats()
        self.assertTrue(stats)
        for task in map(quick_check.task_in_stat, stats):
            self.assertGreaterEqual(task.ticks, 0)


PS = """\
    0     0   2.0 kernel_task
    1     0   0.5 /sbin/launchd
  100     1   1.0 /usr/bin/zsh
  200   100  60.0 /usr/bin/python3
  300   200  90.0 /Users/me/.cargo/bin/cargo
  301   300 100.0 /Users/me/target/debug/deps/taxman-abc
  400     1 100.0 /Users/me/target/debug/examples/perf
  401     1  45.5 /Applications/Microsoft Excel.app/Contents/MacOS/Microsoft Excel
  402     1   4.0 /usr/bin/quiet
  403     1  30.0 /usr/sbin/mds
"""


class Culprits(unittest.TestCase):
    def test_the_busiest_processes_not_ours_are_named_busiest_first(self):
        self.assertEqual(quick_check.busiest(PS, own_pid=200), "perf (100%), Microsoft Excel (46%), mds (30%)")

    def test_our_own_processes_are_left_out_whoever_started_them(self):
        self.assertNotIn("taxman-abc", quick_check.busiest(PS, own_pid=200, count=10))
        self.assertNotIn("cargo", quick_check.busiest(PS, own_pid=200, count=10))
        self.assertIn("taxman-abc", quick_check.busiest(PS, own_pid=400, count=10))

    def test_processes_that_use_next_to_nothing_are_not_named(self):
        self.assertNotIn("quiet", quick_check.busiest(PS, own_pid=200, count=10))

    def test_a_machine_with_nothing_at_work_names_nobody(self):
        self.assertEqual(quick_check.busiest("    1     0   0.0 /sbin/launchd\n", own_pid=200), "")

    def test_this_system_lists_its_processes_or_says_nothing(self):
        self.assertIsInstance(quick_check.who_is_busy(), str)


class Machine:
    """A machine for `wait_until_quiet` and `wait_for_turn`, with a clock of its own.  Each sample takes BUSY_SAMPLE
    seconds and reads the next of `shares` (the last for ever).  Another check holds the lock for the first `held_for`
    tries at it, and each sleep between tries takes the seconds it is given.  `said` is what the waits said."""

    def __init__(self, *shares: Optional[float], held_for: int = 0):
        self.shares = shares
        self.held_for = held_for
        self.now = 0.0
        self.samples = 0
        self.tries = 0
        self.said: List[str] = []

    def clock(self) -> float:
        return self.now

    def sample(self) -> Optional[float]:
        self.now += quick_check.BUSY_SAMPLE
        share = self.shares[min(self.samples, len(self.shares) - 1)]
        self.samples += 1
        return share

    def take(self) -> bool:
        self.tries += 1
        return self.tries > self.held_for

    def sleep(self, seconds: float) -> None:
        self.now += seconds

    def say(self, message: str) -> None:
        self.said.append(message)

    def turn(self) -> Optional[str]:
        return quick_check.wait_for_turn(self.take, self.clock, self.sleep, self.say)

    def waited(self, who=lambda: "", waited_before: float = 0.0) -> Optional[str]:
        return quick_check.wait_until_quiet(self.sample, self.clock, self.say, who, waited_before)


class Waiting(unittest.TestCase):
    def test_a_quiet_machine_is_not_waited_for(self):
        machine = Machine(quick_check.BUSY_LIMIT)
        self.assertIsNone(machine.waited())
        self.assertEqual(machine.samples, 1)
        self.assertEqual(len(machine.said), 1)
        self.assertIn("quiet", machine.said[0])

    def test_a_busy_machine_is_waited_for_until_it_is_quiet_and_the_wait_says_why(self):
        machine = Machine(0.9, 0.6, 0.3, 0.1)
        self.assertIsNone(machine.waited(lambda: "perf (100%)"))
        self.assertEqual(machine.samples, 4)
        self.assertEqual(len(machine.said), 2, machine.said)
        self.assertIn("machine busy", machine.said[0])
        self.assertIn("90.0%", machine.said[0])
        self.assertIn("the busiest: perf (100%)", machine.said[0])
        self.assertIn("quiet", machine.said[1])
        self.assertIn("after 8 s", machine.said[1])

    def test_the_wait_says_it_is_still_busy_once_in_a_while_and_not_after_every_sample(self):
        shares = [0.9] * int(3.5 * quick_check.BUSY_NOTICE / quick_check.BUSY_SAMPLE) + [0.0]
        machine = Machine(*shares)
        self.assertIsNone(machine.waited())
        busy = [message for message in machine.said if "machine busy" in message]
        self.assertEqual(len(busy), 4, machine.said)

    def test_a_machine_that_never_gets_quiet_fails_with_machine_busy_after_the_wait(self):
        machine = Machine(0.8)
        failure = machine.waited(lambda: "perf (100%)")
        self.assertIsNotNone(failure)
        self.assertIn("machine busy", failure)
        self.assertIn("80.0%", failure)
        self.assertIn("perf (100%)", failure)
        self.assertGreaterEqual(machine.now, quick_check.BUSY_WAIT)
        self.assertLess(machine.now, quick_check.BUSY_WAIT + 2 * quick_check.BUSY_SAMPLE)

    def test_the_share_that_is_just_over_the_limit_is_busy_and_the_limit_itself_is_quiet(self):
        self.assertIsNone(Machine(quick_check.BUSY_LIMIT).waited())
        machine = Machine(quick_check.BUSY_LIMIT + 0.001, quick_check.BUSY_LIMIT)
        self.assertIsNone(machine.waited())
        self.assertEqual(machine.samples, 2)

    def test_a_machine_that_cannot_be_measured_is_not_waited_for(self):
        machine = Machine(None)
        self.assertIsNone(machine.waited())
        self.assertEqual(machine.samples, 1)
        self.assertIn("cannot measure", machine.said[0])

    def test_nobody_is_looked_for_while_the_wait_is_quiet(self):
        looked = []
        Machine(0.0).waited(lambda: looked.append(1) or "")
        self.assertEqual(looked, [])

    def test_a_wait_that_follows_seconds_already_waited_gives_up_that_much_sooner(self):
        machine = Machine(0.8)
        self.assertIn("machine busy", machine.waited(waited_before=500.0))
        self.assertEqual(machine.now, quick_check.BUSY_WAIT - 500.0)  # 100 s were left
        self.assertEqual(machine.samples, int((quick_check.BUSY_WAIT - 500.0) / quick_check.BUSY_SAMPLE))

    def test_the_thresholds_make_sense(self):
        self.assertTrue(0 < quick_check.BUSY_LIMIT < 1)
        self.assertLess(quick_check.BUSY_SAMPLE, quick_check.BUSY_NOTICE)
        self.assertLess(quick_check.BUSY_NOTICE, quick_check.BUSY_WAIT)
        self.assertLess(quick_check.LOCK_POLL, quick_check.BUSY_NOTICE)


class Turns(unittest.TestCase):
    """Two checks at once take turns: the second waits for the first, and says so."""

    def test_a_free_lock_is_taken_at_once_without_a_word(self):
        machine = Machine()
        self.assertIsNone(machine.turn())
        self.assertEqual(machine.tries, 1)
        self.assertEqual(machine.now, 0.0)
        self.assertEqual(machine.said, [])

    def test_a_lock_another_check_holds_is_waited_for_until_it_ends_and_the_wait_says_so(self):
        machine = Machine(held_for=5)
        self.assertIsNone(machine.turn())
        self.assertEqual(machine.tries, 6)
        self.assertEqual(machine.now, 5 * quick_check.LOCK_POLL)
        self.assertEqual(len(machine.said), 2, machine.said)
        self.assertIn("another quick check is running; this one waits for it to end before it builds anything",
                      machine.said[0])
        self.assertIn("the other quick check has ended", machine.said[1])

    def test_the_wait_says_the_lock_is_still_held_once_in_a_while_and_not_after_every_try(self):
        machine = Machine(held_for=int(3.5 * quick_check.BUSY_NOTICE / quick_check.LOCK_POLL))
        self.assertIsNone(machine.turn())
        held = [message for message in machine.said if "another quick check is running" in message]
        self.assertEqual(len(held), 4, machine.said)

    def test_a_lock_that_is_never_let_go_fails_after_the_wait_and_nothing_is_built(self):
        machine = Machine(held_for=10 ** 9)
        failure = machine.turn()
        self.assertIsNotNone(failure)
        self.assertIn("another quick check", failure)
        self.assertIn("nothing was built or run", failure)
        self.assertGreaterEqual(machine.now, quick_check.BUSY_WAIT)
        self.assertLess(machine.now, quick_check.BUSY_WAIT + 2 * quick_check.LOCK_POLL)

    def test_the_wait_for_the_lock_and_the_wait_for_a_quiet_machine_count_together_but_not_the_build_between(self):
        machine = Machine(0.9, held_for=50)
        self.assertIsNone(machine.turn())
        waited_for_lock = machine.now
        self.assertEqual(waited_for_lock, 50 * quick_check.LOCK_POLL)
        machine.now += 300.0  # the checks and the build
        before_the_wait = machine.now
        self.assertIn("machine busy", machine.waited(waited_before=waited_for_lock))
        self.assertEqual(machine.now - before_the_wait, quick_check.BUSY_WAIT - waited_for_lock)


# What a check does that crashes while it holds the lock: takes it, says so, and is killed.
HOLDS_THE_LOCK_AND_IS_KILLED = """\
import os, signal, sys
sys.path.insert(0, sys.argv[1])
import quick_check
print(quick_check.take_lock(quick_check.open_lock(sys.argv[2])), flush=True)
os.kill(os.getpid(), signal.SIGKILL)
"""


class Lock(unittest.TestCase):
    """The real lock (`fcntl.flock`) on a file in a temporary directory of its own."""

    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.path = Path(directory.name) / "lock"
        self.open_files = []
        self.addCleanup(lambda: [os.close(fd) for fd in self.open_files])

    def opened(self) -> int:
        fd = quick_check.open_lock(self.path)
        self.open_files.append(fd)
        return fd

    def closed(self, fd: int) -> None:
        self.open_files.remove(fd)
        os.close(fd)

    def test_the_lock_is_a_file_in_the_system_temporary_directory_so_that_every_checkout_shares_it(self):
        self.assertEqual(quick_check.LOCK.parent, Path(tempfile.gettempdir()))

    def test_a_second_check_cannot_take_the_lock_until_the_first_lets_go(self):
        first, second = self.opened(), self.opened()
        self.assertTrue(quick_check.take_lock(first))
        self.assertFalse(quick_check.take_lock(second))
        self.closed(first)
        self.assertTrue(quick_check.take_lock(second))

    def test_a_check_that_crashes_does_not_keep_the_lock(self):
        run = subprocess.run([sys.executable, "-I", "-c", HOLDS_THE_LOCK_AND_IS_KILLED, str(HERE), str(self.path)],
                              capture_output=True, text=True, timeout=60)
        self.assertEqual(run.stdout.strip(), "True", run.stderr)
        self.assertEqual(run.returncode, -signal.SIGKILL)
        self.assertTrue(quick_check.take_lock(self.opened()))


if __name__ == "__main__":
    unittest.main()
