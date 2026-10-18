#!/usr/bin/env python3
"""Unit tests of quick_check.py's judgement of a run (the summary that run_tests.py writes).

usage: python3 -I test_quick_check.py      (or python3 -I -m unittest test_quick_check, from this directory)
"""
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))  # python -I puts neither the script's directory nor the current one on the path

import quick_check  # noqa: E402


def summary(seconds, *tests):
    """What run_tests.py writes: the seconds of the tests and the tests that used the most CPU (id, seconds, cpu)."""
    costliest = [{"id": name, "seconds": wall, "cpu": cpu} for name, wall, cpu in tests]
    return {"seconds": seconds, "costliest": sorted(costliest, key=lambda t: -t["cpu"])}


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

    def test_a_test_that_takes_long_on_a_busy_machine_but_uses_little_cpu_is_not_a_problem(self):
        self.assertEqual(quick_check.problems(summary(1.0, ("waits", 30.0, 0.4))), [])

    def test_the_target_is_below_the_limit(self):
        self.assertLess(quick_check.TOTAL_TARGET, quick_check.TOTAL_LIMIT)


if __name__ == "__main__":
    unittest.main()
