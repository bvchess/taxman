#!/usr/bin/env python3
"""Unit tests of check_results.py's reader of optimal.json, which parses a game when it is first asked for.

usage: python3 -I test_check_results.py      (or python3 -I -m unittest test_check_results, from this directory)
"""
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))  # python -I puts neither the script's directory nor the current one on the path

import check_results  # noqa: E402

SPARSE = """[
  { "n": 3, "score": 3, "moves": [3] },
  { "n": 7, "score": 17, "moves": [7, 4, 6] },
  { "n": 12, "score": 50, "moves": [11, 9, 8, 12, 10] }
]"""


class TableTest(unittest.TestCase):
    def test_a_game_is_the_json_of_its_line(self):
        table = check_results.Table(SPARSE)
        self.assertEqual(table[7], {"n": 7, "score": 17, "moves": [7, 4, 6]})
        self.assertEqual(table.get(12)["moves"], [11, 9, 8, 12, 10])
        self.assertEqual(table[3], json.loads(SPARSE)[0])

    def test_a_game_the_table_lacks_is_missing_as_in_a_dict(self):
        table = check_results.Table(SPARSE)
        self.assertNotIn(5, table)
        self.assertIn(7, table)
        self.assertEqual(table.get(5), None)
        self.assertEqual(table.get(5, {}).get("score"), None)
        with self.assertRaises(KeyError):
            table[5]

    def test_a_table_with_no_games_has_none(self):
        table = check_results.Table("[\n]")
        self.assertNotIn(1, table)
        self.assertEqual(table.get(1), None)

    def test_the_first_games_of_optimal_json_are_those_of_a_full_parse(self):
        lines = check_results.OPTIMAL.read_text().splitlines()
        prefix = "[\n" + "\n".join(lines[1:301]).rstrip(",") + "\n]"
        parsed = {game["n"]: game for game in json.loads(prefix)}
        table = check_results.Table(check_results.OPTIMAL.read_text())
        self.assertEqual(len(parsed), 300)
        for n, game in parsed.items():
            self.assertEqual(table[n], game)
        self.assertIn(len(lines) - 2, table)  # the last game: the first line is the bracket, and so is the last
        self.assertNotIn(len(lines) - 1, table)


if __name__ == "__main__":
    unittest.main()
