#!/usr/bin/env python3
"""Unit tests of check_results.py's reading of the committed file (a gzip file of several members, as
results/optimal.jsonl.gz is), of the table of games it makes, and of the comparison of a file with that table.  They
use small files made here, so they hold for the committed file whatever it contains.

usage: python3 -I test_check_results.py      (or python3 -I -m unittest test_check_results, from this directory)
"""
import contextlib
import gzip
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))  # python -I puts neither the script's directory nor the current one on the path

import check_results  # noqa: E402


def line(n, score, moves, method, proof, prev, **extra):
    """A completed line of schema 5, with the fields that the referee reads."""
    row = {
        "schema": 5, "n": n, "score": score, "moves": moves, "promotions": [], "method": method, "completed": True,
        "proof": proof, "reached_cap": False, "assumes": [], "in_the_bag": 0 if method == "trivial" else None,
        "prev": prev, "same_search": None, "nodes": 0, "setup": None,
    }
    row.update(extra)
    return row


def prime_game(n, score, moves, below):
    """The line of a prime game, built on the game of n - 1, whose line is `below`."""
    prev = {
        "n": n - 1, "score": below["score"], "source": "run", "moves_digest": check_results.digest(below["moves"]),
        "proof": below["proof"], "assumes": [],
    }
    return line(n, score, moves, "prime", "prime_formula", prev)


ONE = line(1, 0, [], "trivial", "trivial", None)
TWO = prime_game(2, 2, [2], ONE)
THREE = prime_game(3, 3, [3], TWO)
GAMES = [ONE, TWO, THREE]


def text_of(rows):
    return "".join(json.dumps(row) + "\n" for row in rows)


class FileCase(unittest.TestCase):
    """A directory for the files of a test."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dir = Path(self.tmp.name)

    def plain(self, name, rows):
        path = self.dir / name
        path.write_text(text_of(rows))
        return path

    def gz(self, name, *members):
        """A gzip file with one member for each list of rows, as `gzip -9 -n >>` appends them."""
        path = self.dir / name
        path.write_bytes(b"".join(gzip.compress(text_of(rows).encode(), 9, mtime=0) for rows in members))
        return path


class ReadTextTest(FileCase):
    def test_a_gz_file_is_read_through_gzip_member_after_member(self):
        path = self.gz("committed.jsonl.gz", GAMES[:2], GAMES[2:])
        self.assertEqual(check_results.read_text(path), text_of(GAMES))

    def test_a_file_without_the_gz_name_is_read_as_it_is(self):
        self.assertEqual(check_results.read_text(self.plain("results.jsonl", GAMES)), text_of(GAMES))


class TableTest(FileCase):
    def test_the_table_is_the_completed_games_of_the_committed_file(self):
        incomplete = dict(line(4, 0, [], "search", "exhausted", None), completed=False)
        older = dict(THREE, n=9, schema=4)
        sources = check_results.Sources(self.gz("committed.jsonl.gz", GAMES[:1], GAMES[1:] + [incomplete, older]))
        self.assertEqual(sorted(sources.table), [1, 2, 3])
        game = sources.table[3]
        self.assertEqual((game.score, game.moves, game.proof, game.assumes), (3, [3], "prime_formula", set()))

    def test_without_a_committed_file_the_table_is_empty(self):
        self.assertEqual(check_results.Sources().table, {})

    def test_a_table_of_its_own_is_the_games_of_its_file_and_the_committed_chain_stays_the_committed_file(self):
        committed = self.gz("committed.jsonl.gz", GAMES[:2])
        table = self.gz("table.jsonl.gz", GAMES)
        sources = check_results.Sources(committed, table=table)
        self.assertEqual((sorted(sources.table), sorted(sources.committed)), ([1, 2, 3], [1, 2]))
        # By default the table is the committed file's games, the same dict; a table that is not given is empty.
        default = check_results.Sources(committed)
        self.assertIs(default.table, default.committed)
        self.assertEqual(check_results.Sources(committed, table=None).table, {})
        self.assertEqual(sources.cite_problems(3, 3, {"source": "optimal"}), ({check_results.assumed_optimal(3)}, []))
        self.assertEqual(
            sources.cite_problems(3, 3, {"source": "strict"})[1],
            ["it cites the committed chain, which has no game of 3 scoring 3"],
        )

    def test_a_game_of_the_table_is_assumed_optimal_in_the_words_of_the_committed_file(self):
        self.assertEqual(
            check_results.assumed_optimal(1300), "the game of 1300 in results/optimal.jsonl.gz is optimal"
        )

    def test_a_citation_of_the_table_holds_for_the_score_of_its_game_only(self):
        sources = check_results.Sources(self.gz("committed.jsonl.gz", GAMES))
        self.assertEqual(
            sources.cite_problems(3, 3, {"source": "optimal"}), ({check_results.assumed_optimal(3)}, [])
        )
        assumes, problems = sources.cite_problems(3, 4, {"source": "optimal"})
        self.assertIsNone(assumes)
        self.assertEqual(problems, ["it cites the table, which has no game of 3 scoring 4"])
        self.assertEqual(sources.cite_problems(3, 3, {"source": "strict"}), (set(), []))


class CheckFileTest(FileCase):
    def check(self, path, sources):
        problems = []
        done, *_rest = check_results.check_file(path, {}, {}, check_results.Divisors(3), problems, sources)
        return done, problems

    def test_a_file_that_agrees_with_the_table_has_no_problem(self):
        sources = check_results.Sources(self.gz("committed.jsonl.gz", GAMES))
        done, problems = self.check(self.plain("results.jsonl", GAMES), sources)
        self.assertEqual((sorted(done), problems), ([1, 2, 3], []))

    def test_a_score_that_the_table_does_not_give_is_a_problem(self):
        table = self.gz("committed.jsonl.gz", GAMES[:2], [dict(THREE, score=5)])
        _done, problems = self.check(self.plain("results.jsonl", GAMES), check_results.Sources(table))
        self.assertEqual(problems, ["results.jsonl line 3 (n=3): scores 3, but the table has 5"])

    def test_the_committed_file_is_checked_as_a_gz_file_against_its_own_games(self):
        path = self.gz("committed.jsonl.gz", GAMES[:1], GAMES[1:])
        done, problems = self.check(path, check_results.Sources(path))
        self.assertEqual((sorted(done), problems), ([1, 2, 3], []))


class MainTest(FileCase):
    def run_main(self, *argv):
        """(exit status, output) of the command with the arguments, and an empty bound and an empty chain of
        heuristics, in place of the committed ones."""
        fm, chain = self.dir / "fm.json", self.dir / "chain.json"
        fm.write_text("{}")
        chain.write_text("[]")
        args = ["check_results.py", "--fm", str(fm), "--chain", str(chain), *map(str, argv)]
        out = io.StringIO()
        with mock.patch.object(sys, "argv", args), contextlib.redirect_stdout(out):
            code = check_results.main()
        return code, out.getvalue()

    def test_a_gz_file_on_the_command_line_is_checked(self):
        path = self.gz("committed.jsonl.gz", GAMES[:1], GAMES[1:])
        code, out = self.run_main(path, "--committed", path, "--bounds", self.dir / "none.jsonl")
        self.assertEqual(code, 0, out)
        self.assertIn("3 completed games, n = 1..3 (0 missing)", out)
        self.assertIn("all checks passed", out)

    def test_a_gz_file_that_the_table_disagrees_with_fails(self):
        committed = self.gz("committed.jsonl.gz", GAMES[:2], [dict(THREE, score=5)])
        path = self.gz("results.jsonl.gz", GAMES)
        code, out = self.run_main(path, "--committed", committed, "--bounds", self.dir / "none.jsonl")
        self.assertEqual(code, 1)
        self.assertIn("results.jsonl.gz line 3 (n=3): scores 3, but the table has 5", out)


if __name__ == "__main__":
    unittest.main()
