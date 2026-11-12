#!/usr/bin/env python3
"""Unit tests of best_known_table.py: the safe reading of a file that is being appended to, the reading of the ledgers,
the order of the effort file, and one small run of the tool whose files are the same bytes the second time.  They use
small files made here, so they hold whatever the committed files contain.

usage: python3 -I test_best_known_table.py      (or python3 -I -m unittest test_best_known_table, from this directory)
"""
import gzip
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))  # python -I puts neither the script's directory nor the current one on the path

import best_known_table as tool  # noqa: E402


def write(path, text):
    Path(path).write_text(text)
    return path


class ReadingTests(unittest.TestCase):
    def setUp(self):
        self.dir = Path(tempfile.mkdtemp(prefix="best-known-table-test-"))
        self.addCleanup(lambda: [p.unlink() for p in self.dir.iterdir()] and self.dir.rmdir())

    def test_a_last_line_that_is_not_finished_is_left_out(self):
        path = write(self.dir / "file.jsonl", '{"n":1}\n{"n":2}\n{"n":3')
        lines, torn = tool.complete_lines(path)
        self.assertEqual(lines, [b'{"n":1}\n', b'{"n":2}\n'])
        self.assertTrue(torn)

    def test_what_is_appended_after_the_file_was_measured_is_not_read(self):
        path = write(self.dir / "file.jsonl", '{"n":1}\n{"n":2}\n{"n":3}\n{"n":4')
        measured = mock.Mock(st_size=len(b'{"n":1}\n{"n":2}\n'))
        with mock.patch.object(tool.os, "fstat", return_value=measured):
            lines, torn = tool.complete_lines(path)
        self.assertEqual(lines, [b'{"n":1}\n', b'{"n":2}\n'])
        self.assertFalse(torn)
        measured = mock.Mock(st_size=len(b'{"n":1}\n{"n":2}\n{"n"'))  # the third line was being written then
        with mock.patch.object(tool.os, "fstat", return_value=measured):
            lines, torn = tool.complete_lines(path)
        self.assertEqual(lines, [b'{"n":1}\n', b'{"n":2}\n'])
        self.assertTrue(torn)

    def test_a_file_that_ends_in_a_newline_has_no_torn_line(self):
        path = write(self.dir / "file.jsonl", '{"n":1}\n\n{"n":2}\n')
        lines, torn = tool.complete_lines(path)
        self.assertEqual(lines, [b'{"n":1}\n', b'{"n":2}\n'])
        self.assertFalse(torn)

    def test_the_lines_of_a_gzip_file_of_several_members_are_read(self):
        path = self.dir / "file.jsonl.gz"
        path.write_bytes(gzip.compress(b'{"n":1}\n') + gzip.compress(b'{"n":2}\n'))
        self.assertEqual([line["n"] for line in tool.json_lines(path)], [1, 2])

    def test_a_line_that_is_not_json_stops_the_tool(self):
        path = write(self.dir / "file.jsonl", '{"n":1}\nnot json\n')
        with self.assertRaises(SystemExit):
            list(tool.json_lines(path))

    def test_the_best_line_of_a_game_is_the_highest_and_the_later_of_two_with_one_score(self):
        rows = [{"n": 11, "score": 5, "moves": [11]}, {"n": 11, "score": 7, "moves": [11, 2]},
                {"n": 11, "score": 7, "moves": [11, 1, 1]}, {"n": 11, "score": 6, "moves": [11]},
                {"n": 5, "score": 9, "moves": [5]}]
        path = write(self.dir / "best.jsonl", "".join(json.dumps(row) + "\n" for row in rows) + '{"n":12,')
        best, count, _, torn = tool.read_best_known(path, 10)
        self.assertEqual(list(best), [11])
        self.assertEqual(list(best[11].moves), [11, 1, 1])
        self.assertEqual((count, torn), (5, True))

    def test_plays_take_the_level_from_the_ledger_or_from_the_path_and_say_if_they_raised_the_game(self):
        rows = "6000\t4\t41\t100\t100\t10:00:00\n6001\t5\t42\t100\t101\t10:01:00\n"
        with_level = write(self.dir / "levels.tsv", rows)
        no_level = write(self.dir / "pilot.tsv", "6000\t41\t100\t99\t10:02:00\n")
        plays = tool.read_plays([str(with_level), f"{no_level}:3"])
        self.assertEqual(plays, [(6000, 4, 41, 0), (6001, 5, 42, 1), (6000, 3, 41, 0)])
        with self.assertRaises(SystemExit):
            tool.read_plays([str(no_level)])

    def test_a_row_that_is_still_being_written_is_not_read(self):
        path = write(self.dir / "levels.tsv", "6000\t4\t41\t100\t100\t10:00:00\n6001\t5\t42\t100")
        self.assertEqual(tool.read_plays([str(path)]), [(6000, 4, 41, 0)])

    def test_the_finder_outcomes_are_none_stopped_or_gain_and_an_error_is_no_answer(self):
        rows = ["7001\t13\tnone\t1\t1\t1.0\t10:00:00", "7002\t13\tstopped\t1\t1\t60.0\t10:00:00",
                "7003\t19\tgain-exact\t1\t2\t5.0\t10:00:00", "7004\t19\terror\t1\t1\t0.0\t10:00:00"]
        path = write(self.dir / "finder.tsv", "\n".join(rows) + "\n")
        self.assertEqual(tool.read_finder([f"{path}:60"]),
                         [(7001, 13, 60, "none"), (7002, 13, 60, "stopped"), (7003, 19, 60, "gain")])
        with self.assertRaises(SystemExit):
            tool.read_finder([str(path)])  # the time limit is not in the ledger: the path must give it
        write(path, "7005\t13\tmystery\t1\t1\t1.0\t10:00:00\n")
        with self.assertRaises(SystemExit):
            tool.read_finder([f"{path}:60"])

    def test_the_effort_has_a_line_for_each_game_tried_in_order_and_counts_what_it_leaves_out(self):
        plays = [(6002, 5, 41, 0), (6001, 4, 42, 1), (6002, 4, 41, 0), (5999, 4, 41, 0)]
        passes = [(6002, 19, 60, "stopped"), (6002, 13, 60, "none"), (6003, 13, 60, "none"), (5260, 13, 60, "none")]
        rows, left_out = tool.effort_rows([6001, 6002, 6003, 6004], plays, passes)
        self.assertEqual(rows, [
            {"n": 6001, "plays": [[4, 42, 1]], "finder": []},
            {"n": 6002, "plays": [[4, 41, 0], [5, 41, 0]], "finder": [[13, 60, "none"], [19, 60, "stopped"]]},
            {"n": 6003, "plays": [], "finder": [[13, 60, "none"]]}])
        self.assertEqual(left_out, [1, 1])


class SmallRunTests(unittest.TestCase):
    """A chain that ends at 99 with a score of 50 and four made-up best-known games, 100 to 103: the tool checks that
    the moves add up to the score, are on the board and open with the largest prime, and not that they follow the
    rules."""

    MOVES = {100: [97, 53], 101: [101, 53], 102: [101, 53, 102], 103: [103, 53, 102]}
    # the sums of the sources of the boards 99 to 103: the change is n at a half-out step and n less the prime before it
    # at a prime
    SOURCES = {99: 50, 100: 150, 101: 154, 102: 256, 103: 258}

    def make_sources(self, root):
        chain = {"n": 99, "score": 50, "in_the_bag": 0, "setup": {"digest": "a"}, "moves": [97]}
        write(root / "chain.jsonl", json.dumps(chain) + "\n")
        write(root / "beyond.jsonl", "")
        best = [{"n": n, "score": sum(moves), "moves": moves, "in_the_bag": 0, "setup": {"digest": f"d{n}"}}
                for n, moves in self.MOVES.items()]
        write(root / "best.jsonl", "".join(json.dumps(line) + "\n" for line in best))
        parity = [{"n": n, "parity": 5000 + n, "ustar_sum": total, "diamond": 0} for n, total in self.SOURCES.items()]
        write(root / "parity.jsonl", "".join(json.dumps(line) + "\n" for line in parity))
        write(root / "plays.tsv", "101\t4\t41\t5\t5\t10:00:00\n")
        write(root / "finder.tsv", "101\t13\tnone\t5\t5\t1.0\t10:00:00\n")

    def run_tool(self, root, out):
        argv = ["best_known_table.py", "--best-known", str(root / "best.jsonl"), "--parity", str(root / "parity.jsonl"),
                "--plays", str(root / "plays.tsv"), "--finder", f"{root / 'finder.tsv'}:60",
                "--chain", str(root / "chain.jsonl"), "--beyond", str(root / "beyond.jsonl"), "--out-dir", str(out)]
        with mock.patch.object(sys, "argv", argv), redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            tool.main()

    def test_the_steps_the_rules_and_the_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.make_sources(root)
            for name in ("first", "second"):
                (root / name).mkdir()
                self.run_tool(root, root / name)
            first = [json.loads(line) for line in gzip.decompress((root / "first" / "best-known.jsonl.gz").read_bytes())
                     .decode().splitlines()]
            self.assertEqual([(row["n"], row["step"], row["rule"]) for row in first],
                             [(100, "half-out", "cap"), (101, "exchange", "prime_formula"), (102, "half-out", "cap"),
                              (103, "exchange", "prime_formula")])
            effort = (root / "first" / "search-effort.jsonl").read_text()
            self.assertEqual(json.loads(effort), {"n": 101, "plays": [[4, 41, 0]], "finder": [[13, 60, "none"]]})
            for name in ("best-known.jsonl.gz", "search-effort.jsonl"):
                self.assertEqual((root / "first" / name).read_bytes(), (root / "second" / name).read_bytes())

    def test_a_score_above_the_parity_bound_stops_the_tool(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.make_sources(root)
            low = [json.loads(line) for line in (root / "parity.jsonl").read_text().splitlines()]
            for line in low:
                line["parity"] = 10
            write(root / "parity.jsonl", "".join(json.dumps(line) + "\n" for line in low))
            (root / "out").mkdir()
            with self.assertRaises(SystemExit):
                self.run_tool(root, root / "out")


if __name__ == "__main__":
    unittest.main()
