#!/usr/bin/env python3
"""Unit tests of sweep_plan.py, on synthetic results files written to a temporary directory.

usage: python3 -I test_sweep_plan.py      (or python3 -I -m unittest test_sweep_plan, from this directory)
"""
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))  # python -I puts neither the script's directory nor the current one on the path

import sweep_plan  # noqa: E402


def done(n, prev=None, proof="exhausted", same=None):
    """A completed line: the fields the plan uses.  prev is (n - 1's score or None, proven, moves)."""
    row = {"n": n, "score": 10 * n, "completed": True, "proof": proof, "moves": [n], "prev": None}
    if prev is not None:
        score, proven, moves = prev
        row["prev"] = {"n": n - 1, "score": score, "source": "run", "moves_digest": sweep_plan.digest(moves),
                       "proof": "exhausted" if proven else None, "assumes": []}
    if same is not None:
        row["same_search"] = {"n": same}
    return row


def stopped(n, best, moves=None, prev=None):
    """An unfinished line with its game.  prev is (n - 1's score, proven, moves), as for done()."""
    row = {"n": n, "score": None, "best_so_far": best, "completed": False, "moves": moves or [n, best], "prev": None}
    if prev is not None:
        score, proven, before = prev
        row["prev"] = {"n": n - 1, "score": score, "source": "run", "moves_digest": sweep_plan.digest(before),
                       "proof": "exhausted" if proven else None, "assumes": []}
    return row


class Splitting(unittest.TestCase):
    def test_setup_seconds_are_the_measured_ones_at_the_measured_boards(self):
        for n, seconds in sweep_plan.SETUP_SECONDS:
            self.assertAlmostEqual(sweep_plan.setup_seconds(n), seconds)
        # Between them it grows, steeply, and past the last it goes on from the last two.
        values = [sweep_plan.setup_seconds(n) for n in (1301, 2200, 2600, 3500, 5000, 6000, 8191)]
        self.assertEqual(values, sorted(values))
        self.assertGreater(sweep_plan.setup_seconds(8191), 150)
        self.assertLess(sweep_plan.setup_seconds(1301), 0.5)

    def test_blocks_tile_the_range_and_carry_about_the_same_work(self):
        for first, last, blocks, limit in [(3441, 8191, 64, 180), (1301, 1340, 4, 1), (5, 5, 3, 10), (3, 12, 64, 1)]:
            split = sweep_plan.split_blocks(first, last, blocks, limit)
            self.assertLessEqual(len(split), min(blocks, last - first + 1))
            self.assertEqual(split[0][0], first)
            self.assertEqual(split[-1][1], last)
            for (a, b), (c, d) in zip(split, split[1:]):
                self.assertEqual(c, b + 1)
                self.assertLessEqual(a, b)
            work = [sum(sweep_plan.game_seconds(n, limit) for n in range(a, b + 1)) for a, b in split]
            if len(split) == blocks and last - first + 1 >= 4 * blocks:
                self.assertLess(max(work) / min(work), 1.5, work)
        # The later games cost more, so the later blocks hold fewer.
        split = sweep_plan.split_blocks(3441, 8191, 64, 180)
        sizes = [b - a + 1 for a, b in split]
        self.assertGreater(sizes[0], sizes[-1])
        self.assertEqual(sum(sizes), 8191 - 3441 + 1)

    def test_the_command_prints_the_blocks(self):
        run = subprocess.run([sys.executable, "-I", str(HERE / "sweep_plan.py"), "blocks", "1301", "1340", "4", "1"],
                             capture_output=True, text=True, check=True)
        rows = [tuple(map(int, line.split())) for line in run.stdout.splitlines()]
        self.assertEqual(rows, sweep_plan.split_blocks(1301, 1340, 4, 1))


class Excerpt(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)

    def write(self, rows, name="results.jsonl"):
        path = Path(self.dir.name) / name
        path.write_text("".join(json.dumps(row) + "\n" for row in rows))
        return path

    def run_excerpt(self, rows, a, b):
        path = self.write(rows)
        out = Path(self.dir.name) / "out.jsonl"
        run = subprocess.run([sys.executable, "-I", str(HERE / "sweep_plan.py"), "excerpt", str(path), str(a), str(b),
                              str(out)], capture_output=True, text=True, check=True)
        count, todo = map(int, run.stdout.split())
        taken = [json.loads(line) for line in out.read_text().splitlines()]
        self.assertEqual(count, len(taken))
        return [(row["n"], row["completed"]) for row in taken], todo

    def test_it_takes_the_block_the_window_before_it_and_what_they_rest_on(self):
        rows = [
            done(1),  # within 8 board sizes before 6, and what game 2 rests on
            done(2, prev=(10, True, [1])),
            done(3, prev=(20, True, [2])),
            done(4, prev=(30, True, [3])),
            stopped(5, 100),  # block 2 (5..6): 5 is unfinished
            stopped(6, 90),
            stopped(6, 95),
            done(7, prev=(95, False, [6, 95]), same=3),  # rests on nothing of 6's but repeats 3, and was seeded
        ]
        # The job over 6..7: the games 6 and 7, the completed games of the 8 before 6 (2, 3, 4), the best unfinished
        # search of 5, and what the completed lines take: 3's prev 2, 2's prev 1, 7's repeat of 3 and its seed's line.
        taken, todo = self.run_excerpt(rows, 6, 7)
        self.assertEqual(taken, [(1, True), (2, True), (3, True), (4, True), (5, False), (6, False), (6, False),
                                 (7, True)])
        self.assertEqual(todo, 1)  # 6 has only unfinished searches; 7 is completed

    def test_a_game_unrelated_to_the_job_is_left_out_and_a_chain_is_followed_to_its_start(self):
        rows = [
            done(1),
            done(50, prev=None),  # a different block, no relation
            done(100),
            done(101, prev=(1010, True, [100])),
            done(102, prev=(1020, True, [101])),
            stopped(103, 5),
        ]
        taken, todo = self.run_excerpt(rows, 103, 103)
        # 103's window holds 102 (and 101, 100 below it) and the line of 103 itself; the lines of 1 and 50 are far.
        self.assertEqual(taken, [(100, True), (101, True), (102, True), (103, False)])
        self.assertEqual(todo, 1)

    def test_the_seed_a_line_names_is_the_unfinished_line_that_has_its_score_and_moves(self):
        rows = [
            stopped(10, 7, moves=[10, 1]),
            stopped(10, 9, moves=[10, 2]),
            stopped(10, 8, moves=[10, 3]),
            done(11, prev=(8, False, [10, 3])),  # built on the third line of 10, not the best
            stopped(12, 3),
        ]
        taken, todo = self.run_excerpt(rows, 12, 12)
        self.assertEqual(taken, [(10, False), (11, True), (12, False)])
        self.assertEqual(todo, 1)
        # The first game of the job takes the best unfinished search of the game before it, and only that one.
        taken, todo = self.run_excerpt(rows[:3], 11, 11)
        self.assertEqual(taken, [(10, False)])
        self.assertEqual(todo, 0)

    def test_the_games_to_search_again_are_those_whose_game_before_has_more_to_give(self):
        improved = sweep_plan.improved
        # The rule of `taxman solve --retry-improved` (solver/window.rs has the same table in its unit test).
        self.assertFalse(improved(None, None) or improved(None, (5, False)))
        self.assertTrue(improved((5, False), None) and improved((5, True), None))
        self.assertFalse(improved((5, True), (5, True)) or improved((9, True), (5, True)))
        self.assertTrue(improved((5, True), (5, False)) and improved((6, False), (5, False)))
        self.assertFalse(improved((5, False), (5, False)) or improved((4, False), (5, False)))
        # In a file: 8 was built on a seed of 5 that was not proven.  The game before it is now better, the same, or proven.
        for before, expected in [(stopped(7, 6), 1), (stopped(7, 5), 0), (done(7), 1), (stopped(7, 4), 0)]:
            rows = [before, stopped(8, 50, prev=(5, False, [7, 5]))]
            taken, todo = self.run_excerpt(rows, 8, 8)
            self.assertEqual(todo, expected, before)
            self.assertEqual(taken[-1], (8, False))
        # The latest of several lines of 8 says what it was built on, whatever any scored: a search started again can end
        # below an earlier one, and has seen the game before it as it is.
        rows = [stopped(7, 6), stopped(8, 50, prev=(5, False, [7, 5])), stopped(8, 40, prev=(6, False, [7, 6]))]
        self.assertEqual(self.run_excerpt(rows, 8, 8)[1], 0)  # the latest, 40, was built on 6, which 7 still has
        rows = [stopped(7, 6), stopped(8, 40, prev=(6, False, [7, 6])), stopped(8, 50, prev=(5, False, [7, 5]))]
        self.assertEqual(self.run_excerpt(rows, 8, 8)[1], 1)  # the latest, 50, was built on 5, and 7 has 6 now

    def test_the_passes_command_cuts_the_excerpt_of_every_block_that_has_work_from_one_reading(self):
        rows = [
            done(1),
            done(2, prev=(10, True, [1])),
            done(3, prev=(20, True, [2])),
            done(4, prev=(30, True, [3])),
            stopped(5, 100),  # block 2 (5..7): 5 had no game before it, and 4 is proven now
            stopped(6, 90, prev=(100, False, [5, 100])),
            stopped(7, 80, prev=(90, False, [6, 90])),
            done(10),  # block 3 (10..11): finished
            done(11, prev=(110, True, [10])),
        ]
        path = self.write(rows)
        plan = Path(self.dir.name) / "plan"
        plan.write_text("1 4\n5 7\n10 11\n")
        outdir = Path(self.dir.name) / "out"
        outdir.mkdir()
        run = subprocess.run([sys.executable, "-I", str(HERE / "sweep_plan.py"), "passes", str(path), str(plan),
                              str(outdir)], capture_output=True, text=True, check=True)
        # Only block 2 has a game to search again; its excerpt is the 8 sizes before it, what they rest on, and itself.
        self.assertEqual(run.stdout.split("\n")[:-1], ["002 5 7 7 1"])
        self.assertEqual(sorted(p.name for p in outdir.iterdir()), ["job-002.count", "job-002.jsonl"])
        self.assertEqual((outdir / "job-002.count").read_text(), "7\n")
        taken = [json.loads(line)["n"] for line in (outdir / "job-002.jsonl").read_text().splitlines()]
        self.assertEqual(taken, [1, 2, 3, 4, 5, 6, 7])

    def test_a_torn_last_line_and_a_line_with_no_game_are_left_out(self):
        path = self.write([done(1), {"n": 2, "completed": False, "invocations": 1, "best_so_far": None}])
        with open(path, "a") as f:
            f.write('{"n":3,"comp')
        out = Path(self.dir.name) / "out.jsonl"
        run = subprocess.run([sys.executable, "-I", str(HERE / "sweep_plan.py"), "excerpt", str(path), "2", "3",
                              str(out)], capture_output=True, text=True, check=True)
        self.assertEqual(run.stdout.split(), ["1", "0"])  # the line of game 1 only


if __name__ == "__main__":
    unittest.main()
