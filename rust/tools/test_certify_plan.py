#!/usr/bin/env python3
"""Unit tests of certify_plan.py, on synthetic inputs written to a temporary directory.

usage: python3 -I test_certify_plan.py      (or python3 -I -m unittest test_certify_plan, from this directory)
"""
import contextlib
import io
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))  # python -I puts neither the script's directory nor the current one on the path

import certify_plan  # noqa: E402

LO = 3300  # the synthetic boards start here, so that the two parts of the report's table (split after 3314) show


def completed(n, proof="exhausted", nodes=1000, threads=8, search=10.0, repeat_of=None):
    """A completed results line, with the fields that the plan uses."""
    row = {"n": n, "score": 10 * n, "completed": True, "proof": proof, "nodes": nodes, "threads": threads,
           "seconds": {"search": search}}
    if repeat_of is not None:
        row["same_search"] = {"n": repeat_of}
    return row


def upper(n, needs=1, relation=None, members=3, secs=1.0):
    """A line of the barred bounds: a game that needs a certificate has relation none and its open members."""
    row = {"n": n, "score": 10 * n, "relation": relation or ("none" if needs else "chain"), "needs": int(needs),
           "secs": secs}
    if needs:
        row["open"] = [[1, 3, 5]] * members
    return row


def problem(n, rejected=2, worth=7, same_as=None, members=3):
    """A line of the extended problems."""
    return {"n": n, "score": 10 * n, "open": members, "rejected": rejected, "worth": worth, "same_as": same_as}


def table_rows(text, heading):
    """The rows of the markdown table under the heading, as lists of cells without the header and the separator."""
    section = text.split(heading)[1].split("\n## ")[0]
    lines = [line for line in section.splitlines() if line.startswith("|")]
    return [[cell.strip().strip("*") for cell in line.strip().strip("|").split("|")] for line in lines[2:]]


class Case(unittest.TestCase):
    """Writes files into a temporary directory and runs the planner on them."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)

    def write(self, name, rows):
        path = self.dir / name
        path.write_text("".join(json.dumps(row) + "\n" for row in rows))
        return str(path)

    def argv(self, results, uppers, problems=None, gaps=None, eras=None, extra=()):
        """The arguments of a run, from lists of rows written to files; the range starts at LO."""
        argv = ["--results", self.write("results.jsonl", results), "--upper", self.write("upper.jsonl", uppers)]
        if problems is not None:
            argv += ["--problems", self.write("problems.jsonl", problems)]
        if gaps is not None:
            argv += ["--gaps", self.write("gaps.jsonl", gaps)]
        for era in eras or [f"{LO}-{LO + 99}:1.0"]:
            argv += ["--era", era]
        return argv + ["--from", str(LO)] + list(extra)

    def plan(self, *args, **kwargs):
        """The Plan of a run (build_plan, without the report)."""
        return certify_plan.build_plan(certify_plan.parse_args(self.argv(*args, **kwargs)), "cmd")

    def games(self, *args, **kwargs):
        """{n: plan line} of a run."""
        return {g["n"]: g for g in self.plan(*args, **kwargs).games}

    def run_argv(self, argv):
        """(exit status, stdout, stderr) of main on the arguments."""
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = certify_plan.main(argv)
        return code, out.getvalue(), err.getvalue()

    def run_main(self, *args, **kwargs):
        return self.run_argv(self.argv(*args, **kwargs))


class EraCalibration(Case):
    def test_c_is_wall_thread_seconds_per_node(self):
        results = [completed(n, nodes=1000, threads=8, search=10.0) for n in range(LO, LO + 10)]
        results += [completed(n, nodes=3000, threads=16, search=20.0) for n in range(LO + 10, LO + 20)]
        uppers = [upper(n, needs=0) for n in range(LO, LO + 20)]
        plan = self.plan(results, uppers, eras=[f"{LO}-{LO + 9}:1.0", f"{LO + 10}-{LO + 19}:2.0"])
        first, second = plan.eras
        self.assertEqual((first.games, first.nodes), (10, 10_000))
        self.assertAlmostEqual(first.c, 1.0 * 3600 * 64 / 10_000)
        self.assertAlmostEqual(first.recorded, 10 * 10.0 * 8 / 3600)
        self.assertEqual((second.games, second.nodes), (10, 30_000))
        self.assertAlmostEqual(second.c, 2.0 * 3600 * 64 / 30_000)
        self.assertAlmostEqual(second.recorded, 10 * 20.0 * 16 / 3600)

    def test_machine_threads_scales_c_and_every_proof_kind_counts(self):
        results = [completed(LO, nodes=2000), completed(LO + 1, proof="cap", nodes=1000),
                   completed(LO + 2, proof="same_search", nodes=0, repeat_of=LO),
                   completed(LO + 3, proof="prime_formula", nodes=0)]
        uppers = [upper(n, needs=0) for n in range(LO, LO + 4)]
        plan = self.plan(results, uppers, eras=[f"{LO}-{LO + 3}:0.5"], extra=["--machine-threads", "8"])
        self.assertEqual(plan.eras[0].nodes, 3000)
        self.assertAlmostEqual(plan.eras[0].c, 0.5 * 3600 * 8 / 3000)

    def test_games_cost_by_their_own_era(self):
        results = [completed(LO, nodes=1000), completed(LO + 1, nodes=1000),
                   completed(LO + 2, nodes=4000), completed(LO + 3, nodes=4000)]
        uppers = [upper(n) for n in range(LO, LO + 4)]
        games = self.games(results, uppers, eras=[f"{LO}-{LO + 1}:1.0", f"{LO + 2}-{LO + 3}:1.0"])
        # Both eras take the same wall hours, so the era of the larger trees has the cheaper node.
        c_low, c_high = 3600 * 64 / 2000, 3600 * 64 / 8000
        self.assertAlmostEqual(games[LO]["expected_thread_hours"], 2.0 * 1000 * c_low / 3600)
        self.assertAlmostEqual(games[LO + 2]["expected_thread_hours"], 2.0 * 4000 * c_high / 3600)

    def test_overhead_scales_every_cost(self):
        results = [completed(LO, nodes=1000)]
        games = self.games(results, [upper(LO)], extra=["--overhead", "3.5"])
        self.assertAlmostEqual(games[LO]["expected_thread_hours"], 3.5 * 1000 * (3600 * 64 / 1000) / 3600)

    def test_a_board_outside_every_era_is_an_error(self):
        results = [completed(n) for n in range(LO, LO + 4)]
        uppers = [upper(n, needs=0) for n in range(LO, LO + 4)]
        code, _out, err = self.run_main(results, uppers, eras=[f"{LO}-{LO + 1}:1.0", f"{LO + 3}-{LO + 3}:1.0"])
        self.assertEqual(code, 1)
        self.assertIn(f"board {LO + 2}", err)
        self.assertIn("no --era", err)

    def test_overlapping_and_malformed_eras_are_errors(self):
        results = [completed(n) for n in range(LO, LO + 4)]
        uppers = [upper(n, needs=0) for n in range(LO, LO + 4)]
        for eras, text in [([f"{LO}-{LO + 2}:1.0", f"{LO + 2}-{LO + 3}:1.0"], "overlap"), (["3300:1.0"], "A-B:HOURS"),
                           ([f"{LO}-{LO + 3}:0"], "positive"), ([f"{LO + 3}-{LO}:1"], "starts after")]:
            code, _out, err = self.run_main(results, uppers, eras=eras)
            self.assertEqual(code, 1, eras)
            self.assertIn(text, err)

    def test_an_era_without_nodes_is_an_error(self):
        results = [completed(n, nodes=0) for n in range(LO, LO + 4)]
        uppers = [upper(n, needs=0) for n in range(LO, LO + 4)]
        code, _out, err = self.run_main(results, uppers)
        self.assertEqual(code, 1)
        self.assertIn("no nodes", err)


class Bases(Case):
    """Each basis of the expected cost, on ten boards whose era is their ten results lines."""

    BOARDS = 10

    def data(self, results=(), uppers=(), without=()):
        """(results, uppers) for LO..LO+9: the given lines, and a searched game of 1000 nodes that needs nothing for
        every board that the lines do not name; the boards in `without` have no results line."""
        rows = {n: completed(n) for n in range(LO, LO + self.BOARDS)}
        rows.update({r["n"]: r for r in results})
        for n in without:
            del rows[n]
        needs = {n: upper(n, needs=0) for n in range(LO, LO + self.BOARDS)}
        needs.update({u["n"]: u for u in uppers})
        return list(rows.values()), list(needs.values())

    def planned(self, results=(), uppers=(), without=(), gaps=None):
        """({n: plan line}, c): c is the thread-seconds per node of the one era, 1 hour on 64 threads."""
        rows, bounds = self.data(results, uppers, without)
        games = self.games(rows, bounds, gaps=gaps, extra=["--to", str(LO + self.BOARDS - 1)])
        return games, 3600 * 64 / sum(r["nodes"] for r in rows)

    def test_chain(self):
        games, c = self.planned([completed(LO, nodes=1000, threads=64, search=7.0)], [upper(LO)])
        g = games[LO]
        self.assertEqual((g["basis"], g["proof"], g["chain_nodes"], g["chain_seconds"], g["chain_threads"]),
                         ("chain", "exhausted", 1000, 7.0, 64))
        self.assertAlmostEqual(g["expected_thread_hours"], 2.0 * 1000 * c / 3600)
        self.assertAlmostEqual(g["expected_thread_hours_recorded"], 2.0 * 7.0 * 64 / 3600)
        self.assertEqual((g["cost_nodes"], g["cost_seconds"], g["cost_threads"]), (1000, 7.0, 64))
        self.assertEqual((g["open"], g["rejected"], g["relation"], g["needs"]), (3, None, "none", 1))
        self.assertEqual((g["m"], g["source"], g["share"], g["reason"]), (None, None, None, None))
        self.assertIs(g["share_refused"], False)

    def test_source_of_a_repeat_through_a_chain_of_repeats(self):
        # 3302 is searched; 3303 repeats it and needs nothing; 3304 repeats 3303 and needs a certificate.
        results = [completed(LO + 2, nodes=2000, threads=32, search=9.0),
                   completed(LO + 3, proof="same_search", nodes=0, repeat_of=LO + 2),
                   completed(LO + 4, proof="same_search", nodes=0, repeat_of=LO + 3, search=0.0)]
        games, c = self.planned(results, [upper(LO + 2), upper(LO + 4)])
        g = games[LO + 4]
        self.assertEqual((g["basis"], g["proof"], g["m"], g["source"], g["source_proof"], g["source_exhausted"]),
                         ("source", "same_search", LO + 3, LO + 2, "exhausted", True))
        self.assertEqual((g["chain_nodes"], g["cost_nodes"], g["cost_seconds"], g["cost_threads"]),
                         (None, 2000, 9.0, 32))
        self.assertAlmostEqual(g["expected_thread_hours"], 2.0 * 2000 * c / 3600)
        self.assertAlmostEqual(g["expected_thread_hours_recorded"], 2.0 * 9.0 * 32 / 3600)
        self.assertEqual(games[LO + 2]["basis"], "chain")

    def test_gap_model_for_a_source_proven_at_its_ceiling(self):
        results = [completed(LO, proof="cap", nodes=1000),
                   completed(LO + 1, proof="same_search", nodes=0, repeat_of=LO)]
        games, c = self.planned(results, [upper(LO + 1)], gaps=[{"n": LO, "gap": 800.0}, {"n": LO + 1, "gap": 5.0}])
        nodes = 10 ** (2.30 + 0.0479 * 800.0 / (LO + 1) ** 0.3 + 0.06)
        g = games[LO + 1]
        self.assertEqual((g["basis"], g["source"], g["source_proof"], g["source_exhausted"], g["gap"]),
                         ("gap model", LO, "cap", False, 800.0))
        self.assertAlmostEqual(g["cost_nodes"], nodes)
        self.assertAlmostEqual(g["expected_thread_hours"], 2.0 * nodes * c / 3600)
        self.assertEqual(g["expected_thread_hours_recorded"], g["expected_thread_hours"])
        self.assertIsNone(g["cost_seconds"])
        self.assertAlmostEqual(certify_plan.gap_model_nodes(3000, 500.0), 10 ** (2.36 + 0.0479 * 500 / 3000**0.3))

    def test_unknown_when_an_input_is_missing(self):
        results = [completed(LO, proof="cap"),
                   completed(LO + 1, proof="same_search", nodes=0, repeat_of=LO),  # no gap for LO
                   completed(LO + 3, proof="same_search", nodes=0, repeat_of=LO - 50),  # source not in the results
                   completed(LO + 4, proof="prime_formula", nodes=0)]  # a proof with no search to price
        bounds = [upper(n) for n in range(LO + 1, LO + 5)]  # LO + 2 has no results line
        games, _c = self.planned(results, bounds, without=[LO + 2], gaps=[{"n": LO + 7, "gap": 1.0}])
        for n in range(LO + 1, LO + 5):
            self.assertEqual(games[n]["basis"], "unknown", n)
            self.assertIsNone(games[n]["expected_thread_hours"], n)
            self.assertIsNone(games[n]["expected_thread_hours_recorded"], n)
        self.assertIn("--gaps", games[LO + 1]["reason"])
        self.assertIn("no completed line", games[LO + 2]["reason"])
        self.assertIn(f"board {LO - 50}", games[LO + 3]["reason"])
        self.assertIn("prime_formula", games[LO + 4]["reason"])
        rows, uppers = self.data(results, bounds, without=[LO + 2])
        code, out, _err = self.run_main(rows, uppers, extra=["--to", str(LO + 9)])
        self.assertEqual(code, 0)
        listing = out.split("## Games of unknown cost")[1].split("## Games whose share is refused")[0]
        for n in range(LO + 1, LO + 5):
            self.assertIn(f"- {n}:", listing)
        self.assertIn("(4 of unknown cost are not in it)", out)

    def test_games_that_need_nothing_cost_nothing(self):
        games, _c = self.planned([completed(LO, proof="cap", nodes=0)], [])
        for n in (LO, LO + 1):
            g = games[n]
            self.assertEqual((g["basis"], g["expected_thread_hours"], g["expected_thread_hours_recorded"]),
                             ("none", 0.0, 0.0))
            self.assertEqual((g["proof"], g["open"], g["rejected"], g["share_refused"]), (None,) * 4)
        self.assertEqual(games[LO]["relation"], "ceiling")

    def test_relation_words(self):
        bounds = [upper(LO, needs=0, relation="prime"), upper(LO + 1, needs=0, relation="chain"), upper(LO + 2)]
        games, _c = self.planned([], bounds)
        self.assertEqual([games[n]["relation"] for n in (LO, LO + 1, LO + 2)], ["prime", "ceiling", "none"])


class Shares(Case):
    """Shares from the problems."""

    def setUp(self):
        super().setUp()
        self.boards = range(LO, LO + 8)
        # 3300 is searched; 3301..3307 each repeat the board below them.
        self.results = [completed(LO, nodes=1000)] + [
            completed(n, proof="same_search", nodes=0, repeat_of=n - 1) for n in self.boards[1:]
        ]
        self.uppers = [upper(n) for n in self.boards]

    def test_equal_worths_share_and_a_chain_of_shares_costs_one_search(self):
        problems = [problem(LO), problem(LO + 1, same_as=LO), problem(LO + 2, same_as=LO + 1)]
        problems += [problem(n, worth=n) for n in range(LO + 3, LO + 8)]
        games = self.games(self.results, self.uppers, problems=problems)
        self.assertEqual([games[n]["share"] for n in (LO, LO + 1, LO + 2)], [None, LO, LO + 1])
        self.assertEqual([games[n]["basis"] for n in (LO + 1, LO + 2)], ["shared", "shared"])
        self.assertEqual(games[LO + 2]["expected_thread_hours"], 0.0)
        self.assertEqual(games[LO + 2]["expected_thread_hours_recorded"], 0.0)
        self.assertEqual(games[LO + 1]["rejected"], 2)
        self.assertFalse(games[LO + 1]["share_refused"])
        # A shared game keeps what the results say of it.
        self.assertEqual((games[LO + 2]["proof"], games[LO + 2]["source"]), ("same_search", LO))
        self.assertEqual(games[LO + 3]["basis"], "source")

    def test_a_refused_share_is_costed_and_reported(self):
        problems = [problem(LO), problem(LO + 1, same_as=LO, worth=99)]
        problems += [problem(n, worth=n) for n in range(LO + 2, LO + 8)]
        games = self.games(self.results, self.uppers, problems=problems)
        g = games[LO + 1]
        self.assertTrue(g["share_refused"])
        self.assertIsNone(g["share"])
        self.assertEqual(g["basis"], "source")
        self.assertGreater(g["expected_thread_hours"], 0)
        code, out, _err = self.run_main(self.results, self.uppers, problems=problems)
        self.assertEqual(code, 0)
        refused = out.split("## Games whose share is refused")[1]
        self.assertIn(f"- {LO + 1} (same_as {LO})", refused)
        self.assertNotIn(f"- {LO + 2}", refused)

    def test_no_share_with_a_game_that_needs_no_certificate_or_is_not_planned(self):
        uppers = [upper(LO), upper(LO + 1, needs=0), upper(LO + 2)] + [upper(n) for n in range(LO + 3, LO + 8)]
        problems = [problem(LO), problem(LO + 2, same_as=LO + 1)]  # LO + 1 needs none
        problems += [problem(n, worth=n) for n in range(LO + 3, LO + 7)]
        problems.append(problem(LO + 7, same_as=LO - 4))  # a board below the range planned
        games = self.games(self.results, uppers, problems=problems)
        self.assertEqual((games[LO + 2]["share"], games[LO + 2]["share_refused"]), (None, False))
        self.assertEqual((games[LO + 7]["share"], games[LO + 7]["share_refused"]), (None, False))
        self.assertEqual(games[LO + 2]["same_as"], LO + 1)
        self.assertEqual(games[LO + 2]["basis"], "source")

    def test_a_game_whose_problem_line_is_missing_cannot_share(self):
        problems = [problem(LO), problem(LO + 2, same_as=LO + 1)]  # no line for LO + 1, which needs a certificate
        games = self.games(self.results, self.uppers, problems=problems)
        self.assertEqual((games[LO + 2]["share"], games[LO + 2]["share_refused"]), (None, False))
        self.assertIsNone(games[LO + 1]["rejected"])

    def test_without_problems_nothing_is_shared(self):
        games = self.games(self.results, self.uppers)
        self.assertTrue(all(g["share"] is None and g["rejected"] is None for g in games.values()))

    def test_a_problem_that_contradicts_the_bounds_is_an_error(self):
        for bad, text in [
            (dict(problem(LO), score=1), "score"),
            (dict(problem(LO), open=4), "open"),
            (dict(problem(LO), same_as=LO), "same_as"),
            ({k: v for k, v in problem(LO).items() if k != "worth"}, "worth"),
            (problem(LO, rejected=-1), "rejected"),
        ]:
            code, _out, err = self.run_main(self.results, self.uppers, problems=[bad])
            self.assertEqual(code, 1, text)
            self.assertIn(text, err)
            self.assertIn("problems.jsonl line 1", err)
        uppers = [upper(LO, needs=0)] + self.uppers[1:]
        code, _out, err = self.run_main(self.results, uppers, problems=[problem(LO)])
        self.assertEqual(code, 1)
        self.assertIn("needs no certificate", err)

    def test_a_problem_in_two_files_must_agree(self):
        one = self.write("one.jsonl", [problem(LO)])
        same = self.write("same.jsonl", [problem(LO)])
        other = self.write("other.jsonl", [problem(LO, worth=8)])
        argv = self.argv(self.results, self.uppers)
        self.assertEqual(self.run_argv(argv + ["--problems", one, same])[0], 0)
        code, _out, err = self.run_argv(argv + ["--problems", one, other])
        self.assertEqual(code, 1)
        self.assertIn("other.jsonl line 1", err)


class Lists(Case):
    def test_groups_stay_whole_and_in_order(self):
        games = []
        for n, share, hours in [(1, None, 5.0), (2, None, 1.0), (3, 1, 0.0), (4, 3, 0.0), (5, 2, 0.0), (6, None, 3.0)]:
            games.append({"n": n, "needs": 1, "share": share, "expected_thread_hours": hours})
        games.append({"n": 7, "needs": 0, "share": None, "expected_thread_hours": 0.0})
        lists = {entry["name"]: entry for entry in certify_plan.make_lists(games, 2)}
        # Three heads, fewer than 5: all in the pilot list, their groups whole (1, 3, 4 and 2, 5), in increasing n.
        self.assertEqual(lists["pilot"]["games"], [1, 2, 3, 4, 5, 6])
        self.assertEqual(lists["pilot"]["groups"], 3)
        self.assertEqual(lists["dear"]["games"], [])
        self.assertEqual([lists["rest_1"]["games"], lists["rest_2"]["games"]], [[], []])
        self.assertEqual(list(lists), ["pilot", "dear", "rest_1", "rest_2"])

    def test_pilot_dear_and_rest(self):
        # 25 heads, head k costing k hours; boards 100 and 101 share with 3 and 102 with 100: one group of 3.
        games = [{"n": k, "needs": 1, "share": None, "expected_thread_hours": float(k)} for k in range(1, 26)]
        games += [{"n": 100, "needs": 1, "share": 3, "expected_thread_hours": 0.0},
                  {"n": 101, "needs": 1, "share": 3, "expected_thread_hours": 0.0},
                  {"n": 102, "needs": 1, "share": 100, "expected_thread_hours": 0.0}]
        lists = {entry["name"]: entry for entry in certify_plan.make_lists(games, 2)}
        self.assertEqual(lists["pilot"]["games"], [21, 22, 23, 24, 25])
        self.assertEqual(lists["dear"]["games"], list(range(6, 21)))
        # The heads 1..5, in increasing n, each to the list with the smaller total: 1 to list 1 (a tie), 2 to list 2
        # (0 < 1), 3 with its group to list 1 (1 < 2), 4 to list 2 (2 < 4), 5 to list 1 (4 < 6).
        self.assertEqual(lists["rest_1"]["games"], [1, 3, 5, 100, 101, 102])
        self.assertEqual(lists["rest_2"]["games"], [2, 4])
        self.assertEqual([lists["rest_1"]["hours"], lists["rest_2"]["hours"]], [9.0, 6.0])
        self.assertEqual([lists["pilot"]["hours"], lists["dear"]["hours"]], [115.0, sum(range(6, 21))])

    def test_a_group_goes_with_its_head_into_the_pilot(self):
        games = [{"n": k, "needs": 1, "share": None, "expected_thread_hours": float(100 - k)} for k in range(1, 30)]
        games += [{"n": 200, "needs": 1, "share": 2, "expected_thread_hours": 0.0}]
        lists = {entry["name"]: entry for entry in certify_plan.make_lists(games, 3)}
        self.assertEqual(lists["pilot"]["games"], [1, 2, 3, 4, 5, 200])
        self.assertEqual(lists["pilot"]["groups"], 5)
        placed = [n for entry in lists.values() for n in entry["games"]]
        self.assertEqual(sorted(placed), sorted(g["n"] for g in games))

    def test_ties_go_to_the_lowest_list_and_the_smallest_total_wins(self):
        # 20 dear heads first (cost 1000 + k), then rest heads of cost 5, 1, 1, 1, 1, 1, 1 in increasing n.
        games = [{"n": k, "needs": 1, "share": None, "expected_thread_hours": 1000.0 + k} for k in range(1, 21)]
        costs = [5, 1, 1, 1, 1, 1, 1]
        games += [{"n": 100 + i, "needs": 1, "share": None, "expected_thread_hours": float(c)}
                  for i, c in enumerate(costs)]
        lists = {entry["name"]: entry for entry in certify_plan.make_lists(games, 2)}
        self.assertEqual(lists["rest_1"]["games"], [100, 106])
        self.assertEqual(lists["rest_2"]["games"], [101, 102, 103, 104, 105])

    def test_spread_takes_the_heads_in_increasing_n_or_the_dearest_first(self):
        members = {n: [n] for n in range(1, 7)}
        cost = {1: 1.0, 2: 1.0, 3: 1.0, 4: 1.0, 5: 5.0, 6: 5.0}
        # In increasing n: 1 and 2 to the two lists, 3 to list 1 (a tie), 4 to list 2, 5 to list 1, 6 to list 2.
        self.assertEqual(certify_plan.spread(list(members), members, cost, 2), [[1, 3, 5], [2, 4, 6]])
        # The dearest first: 5 and 6 to the two lists, then the four of cost 1 one after the other.
        dearest = certify_plan.spread(list(members), members, cost, 2, dearest_first=True)
        self.assertEqual(dearest, [[5, 1, 3], [6, 2, 4]])

    def test_group_games_gives_the_groups_and_what_each_game_costs(self):
        games = [{"n": 1, "needs": 1, "share": None, "expected_thread_hours": 4.0},
                 {"n": 2, "needs": 0, "share": None, "expected_thread_hours": 0.0},
                 {"n": 3, "needs": 1, "share": 1, "expected_thread_hours": 0.0},
                 {"n": 4, "needs": 1, "share": 3, "expected_thread_hours": 0.0},
                 {"n": 5, "needs": 1, "share": None, "expected_thread_hours": None}]
        members, cost = certify_plan.group_games(games)
        self.assertEqual(members, {1: [1, 3, 4], 5: [5]})
        self.assertEqual(cost, {1: 4.0, 3: 0.0, 4: 0.0, 5: 0.0})
        self.assertEqual(certify_plan.dearest_heads(members, cost), [1, 5])

    def test_an_unknown_cost_ranks_and_weighs_as_zero(self):
        games = [{"n": k, "needs": 1, "share": None, "expected_thread_hours": float(k)} for k in range(1, 7)]
        games.append({"n": 7, "needs": 1, "share": None, "expected_thread_hours": None})
        lists = {entry["name"]: entry for entry in certify_plan.make_lists(games, 1)}
        self.assertEqual(lists["pilot"]["games"], [2, 3, 4, 5, 6])
        self.assertEqual(lists["dear"]["games"], [1, 7])

    def test_files_cover_every_game_that_needs_a_certificate_once(self):
        count = 50
        boards = range(LO, LO + count)
        # The cost falls with n.  Every board ending in 9 needs nothing; 3303 and 3304 repeat 3302 and share its
        # certificate (3304 through 3303).
        results = [completed(n, nodes=1000 * (LO + count - n)) for n in boards]
        for n in (LO + 3, LO + 4):
            results[n - LO] = completed(n, proof="same_search", nodes=0, repeat_of=LO + 2)
        uppers = [upper(n, needs=0) if n % 10 == 9 else upper(n) for n in boards]
        needing = [n for n in boards if n % 10 != 9]
        problems = {n: problem(n, worth=n) for n in needing}
        problems[LO + 2] = problem(LO + 2, worth=LO + 2)
        problems[LO + 3] = problem(LO + 3, same_as=LO + 2, worth=LO + 2)
        problems[LO + 4] = problem(LO + 4, same_as=LO + 3, worth=LO + 2)
        out_dir = self.dir / "out"
        code, _out, err = self.run_main(results, uppers, problems=[problems[n] for n in needing],
                                        extra=["--out-dir", str(out_dir), "--to", str(LO + count - 1)])
        self.assertEqual(code, 0, err)
        files = sorted(p.name for p in (out_dir / "lists").iterdir())
        self.assertEqual(files, ["dear.txt", "pilot.txt", "rest_1.txt", "rest_2.txt", "rest_3.txt", "rest_4.txt"])
        placed = {}
        listed = {}
        for name in files:
            lines = (out_dir / "lists" / name).read_text().splitlines()
            header = [line for line in lines if line.startswith("# ")]
            listed[name] = [int(line) for line in lines if not line.startswith("# ")]
            self.assertEqual(lines[: len(header)], header, name)
            self.assertEqual(len(header), 4, name)
            self.assertIn("expected thread-hours", header[2])
            self.assertTrue(header[3].startswith("# command: certify_plan.py"))
            self.assertEqual(listed[name], sorted(listed[name]), name)
            for n in listed[name]:
                self.assertNotIn(n, placed)
                placed[n] = name
        self.assertEqual(sorted(placed), needing)
        # The dearest heads are the lowest boards: 3300, 3301, 3302 (its group 3303, 3304 with it), 3305, 3306.
        self.assertEqual(listed["pilot.txt"], [LO + k for k in range(7)])
        self.assertEqual(listed["dear.txt"], [n for n in needing if n >= LO + 7][:15])
        # The rest lists are balanced to within the dearest game among them.
        plan = {json.loads(line)["n"]: json.loads(line) for line in (out_dir / "plan.jsonl").read_text().splitlines()}
        totals = [sum(plan[n]["expected_thread_hours"] for n in listed[f"rest_{i}.txt"]) for i in range(1, 5)]
        biggest = max(plan[n]["expected_thread_hours"] for i in range(1, 5) for n in listed[f"rest_{i}.txt"])
        self.assertLessEqual(max(totals) - min(totals), biggest)
        self.assertGreater(min(totals), 0)
        # The first heads of the rest go one to each list (3324 is dearest), then the cheapest total takes the next.
        self.assertEqual([listed[f"rest_{i}.txt"][0] for i in range(1, 5)], [LO + 24, LO + 25, LO + 26, LO + 27])
        self.assertEqual(listed["rest_4.txt"][1], LO + 28)

    def test_rerun_removes_the_lists_of_an_earlier_run(self):
        results = [completed(n) for n in range(LO, LO + 30)]
        uppers = [upper(n) for n in range(LO, LO + 30)]
        out_dir = self.dir / "out"
        for lists, expected in [(4, 6), (2, 4)]:
            code, _out, err = self.run_main(results, uppers, extra=["--out-dir", str(out_dir), "--lists", str(lists)])
            self.assertEqual(code, 0, err)
            self.assertEqual(len(list((out_dir / "lists").iterdir())), expected)


class Inputs(Case):
    def test_a_board_without_an_upper_line_is_an_error(self):
        results = [completed(n) for n in range(LO, LO + 6)]
        uppers = [upper(n) for n in range(LO, LO + 6) if n != LO + 3]
        code, out, err = self.run_main(results, uppers)
        self.assertEqual(code, 1)
        self.assertEqual(out, "")
        self.assertIn(f"board {LO + 3}", err)
        self.assertIn("--upper", err)

    def test_a_board_below_the_range_may_lack_an_upper_line(self):
        results = [completed(n) for n in range(LO - 3, LO + 4)]
        uppers = [upper(n) for n in range(LO, LO + 4)]
        code, _out, err = self.run_main(results, uppers, eras=[f"{LO - 3}-{LO + 3}:1.0"])
        self.assertEqual(code, 0, err)

    def test_a_score_that_differs_from_the_results_is_an_error(self):
        results = [completed(n) for n in range(LO, LO + 4)]
        uppers = [upper(n) for n in range(LO, LO + 4)]
        uppers[2]["score"] += 1
        code, _out, err = self.run_main(results, uppers)
        self.assertEqual(code, 1)
        self.assertIn(f"board {LO + 2}", err)
        self.assertIn("upper.jsonl line 3", err)

    def test_a_board_in_two_upper_files_must_have_equal_lines(self):
        argv = self.argv([completed(n) for n in range(LO, LO + 4)], [upper(n) for n in range(LO, LO + 4)])
        first = argv[argv.index("--upper") + 1]
        same = self.write("same.jsonl", [upper(LO + 1, secs=9.9), upper(LO + 2)])  # a different timing is no fact
        other = self.write("other.jsonl", [upper(LO + 1, members=4)])
        self.assertEqual(self.run_argv(argv + ["--upper", first, same])[0], 0)
        code, _out, err = self.run_argv(argv + ["--upper", first, other])
        self.assertEqual(code, 1)
        self.assertIn(f"board {LO + 1}", err)
        self.assertIn("other.jsonl line 1", err)
        self.assertIn("differs", err)

    def test_a_later_results_file_replaces_an_earlier_one_and_only_completed_lines_count(self):
        first = self.write("first.jsonl", [completed(LO, nodes=1000), completed(LO + 1, nodes=1000),
                                           dict(completed(LO + 2, nodes=1_000_000), completed=False)])
        second = self.write("second.jsonl", [completed(LO + 1, nodes=3000)])
        results = certify_plan.load_results([first, second])
        self.assertEqual({n: r.nodes for n, r in results.items()}, {LO: 1000, LO + 1: 3000})

    def test_a_malformed_line_is_an_error_naming_file_and_line(self):
        for text, expected in [('{"n": 1}\n\n{"n": \n', "bad.jsonl line 3: not JSON"),
                               ("[1, 2]\n", "bad.jsonl line 1: not a JSON object")]:
            path = self.dir / "bad.jsonl"
            path.write_text(text)
            argv = self.argv([completed(LO)], [upper(LO, needs=0)])
            argv[argv.index("--results") + 1] = str(path)
            code, out, err = self.run_argv(argv)
            self.assertEqual((code, out), (1, ""))
            self.assertIn(expected, err)

    def test_a_missing_file_is_an_error(self):
        argv = self.argv([completed(LO)], [upper(LO, needs=0)])
        argv[argv.index("--upper") + 1] = str(self.dir / "nowhere.jsonl")
        code, _out, err = self.run_argv(argv)
        self.assertEqual(code, 1)
        self.assertIn("nowhere.jsonl", err)

    def test_a_missing_or_wrongly_typed_field_is_an_error(self):
        for row, text in [
            ({k: v for k, v in completed(LO).items() if k != "nodes"}, "no field 'nodes'"),
            (dict(completed(LO), nodes="many"), "'nodes' is 'many'"),
            (dict(completed(LO), nodes=True), "'nodes' is True"),
            (dict(completed(LO), proof="mystery"), "unknown proof"),
            (dict(completed(LO, proof="same_search"), same_search={"n": LO + 5}), "not below"),
            (dict(completed(LO), seconds={}), "no field 'search'"),
        ]:
            code, _out, err = self.run_main([row], [upper(LO, needs=0)])
            self.assertEqual(code, 1, text)
            self.assertIn(text, err)
            self.assertIn("results.jsonl line 1", err)
        for row, text in [
            (dict(upper(LO), needs=2), "not 0 or 1"),
            (dict(upper(LO), relation="odd"), "unknown relation"),
            ({k: v for k, v in upper(LO).items() if k != "open"}, "no field 'open'"),
        ]:
            code, _out, err = self.run_main([completed(LO)], [row])
            self.assertEqual(code, 1, text)
            self.assertIn(text, err)
            self.assertIn("upper.jsonl line 1", err)

    def test_gap_lines_must_agree(self):
        results = [completed(LO)]
        code, _out, err = self.run_main(results, [upper(LO, needs=0)], gaps=[{"n": LO, "gap": 1}, {"n": LO, "gap": 2}])
        self.assertEqual(code, 1)
        self.assertIn("gaps.jsonl line 2", err)
        code, _out, err = self.run_main(results, [upper(LO, needs=0)], gaps=[{"n": LO, "gap": 1}, {"n": LO, "gap": 1}])
        self.assertEqual(code, 0, err)

    def test_default_range_runs_from_2176_to_the_largest_board_of_the_results(self):
        results = [completed(n) for n in range(2170, 2181)]
        uppers = [upper(n) for n in range(2176, 2181)]
        argv = ["--results", self.write("r.jsonl", results), "--upper", self.write("u.jsonl", uppers),
                "--era", "2170-2180:1"]
        code, out, err = self.run_argv(argv)
        self.assertEqual(code, 0, err)
        self.assertIn("range: 2176..2180 (5 boards)", out)

    def test_options_must_be_sensible(self):
        for extra in (["--overhead", "0"], ["--lists", "0"], ["--machine-threads", "0"]):
            code, _out, err = self.run_main([completed(LO)], [upper(LO, needs=0)], extra=extra)
            self.assertEqual(code, 1, extra)
            self.assertIn("positive", err)


class Report(Case):
    def test_bands_parts_and_totals(self):
        boards = range(LO, LO + 31)  # 3300..3330: the parts split after 3314
        results, uppers = [], []
        for n in boards:
            if n % 10 == 0:
                uppers.append(upper(n, needs=0))
                results.append(completed(n, proof="cap", nodes=0))
            elif n % 10 == 1:
                uppers.append(upper(n, needs=0, relation="prime"))
                results.append(completed(n, proof="prime_formula", nodes=0))
            elif n % 10 == 5:
                uppers.append(upper(n))
                results.append(completed(n, proof="same_search", nodes=0, repeat_of=n - 1))
            else:
                uppers.append(upper(n))
                results.append(completed(n, nodes=100))
        code, out, err = self.run_main(results, uppers, extra=["--to", str(boards[-1])])
        self.assertEqual(code, 0, err)
        rows = {row[0]: row for row in table_rows(out, "## By band of 100 boards")}
        self.assertEqual(list(rows), ["3300-3330", "3300-3314", "3315-3330", "all 3300-3330"])
        # boards, need a certificate, repeats, shared, searched, prime, ceiling, expected thread-hours
        for label, lo, hi in [("3300-3330", 3300, 3330), ("3300-3314", 3300, 3314), ("3315-3330", 3315, 3330),
                              ("all 3300-3330", 3300, 3330)]:
            span = range(lo, hi + 1)
            counts = [int(cell.replace(",", "")) for cell in rows[label][1:8]]
            expected = [len(span),
                        sum(n % 10 not in (0, 1) for n in span),
                        sum(n % 10 == 5 for n in span),
                        0,
                        sum(n % 10 in (2, 3, 4, 6, 7, 8, 9) for n in span),
                        sum(n % 10 == 1 for n in span),
                        sum(n % 10 == 0 for n in span)]
            self.assertEqual(counts, expected, label)
        hours = {label: float(row[8].replace(",", "")) for label, row in rows.items()}
        self.assertAlmostEqual(hours["3300-3314"] + hours["3315-3330"], hours["all 3300-3330"], places=1)

    def test_the_first_band_is_labelled_by_the_part_of_it_planned(self):
        results = [completed(2176 + i) for i in range(30)]
        uppers = [upper(2176 + i) for i in range(30)]
        argv = ["--results", self.write("r.jsonl", results), "--upper", self.write("u.jsonl", uppers),
                "--era", "2176-2205:1", "--to", "2205"]
        code, out, err = self.run_argv(argv)
        self.assertEqual(code, 0, err)
        # The range has only the first part, which would repeat the total of all.
        self.assertEqual([row[0] for row in table_rows(out, "## By band")], ["2176-2199", "2200-2205", "all 2176-2205"])

    def test_the_twenty_dearest_are_sorted_with_their_columns(self):
        results = [completed(n, nodes=1000 + n, threads=64, search=12.5) for n in range(LO, LO + 30)]
        uppers = [upper(n) for n in range(LO, LO + 30)]
        problems = [problem(n, rejected=n - LO) for n in range(LO, LO + 30)]
        code, out, err = self.run_main(results, uppers, problems=problems, extra=["--to", str(LO + 29)])
        self.assertEqual(code, 0, err)
        rows = table_rows(out, "## The 20 dearest games")
        self.assertEqual(len(rows), 20)
        self.assertEqual(rows[0][:4], [str(LO + 29), "exhausted", "chain", "itself"])
        self.assertEqual(rows[0][4:6], ["3", "29"])
        self.assertEqual(rows[0][6:9], [f"{1000 + LO + 29:,}", "12.5", "64"])
        ns = [int(row[0]) for row in rows]
        self.assertEqual(ns, sorted(ns, reverse=True))
        # thread-hours, then hours on 64 threads
        self.assertAlmostEqual(float(rows[0][9].replace(",", "")) / 64, float(rows[0][10]), places=1)

    def test_the_dearest_table_names_the_source_of_a_cost(self):
        results = [completed(LO, nodes=5000), completed(LO + 1, proof="same_search", nodes=0, repeat_of=LO)]
        code, out, err = self.run_main(results, [upper(LO), upper(LO + 1)], extra=["--to", str(LO + 1)])
        self.assertEqual(code, 0, err)
        rows = {row[0]: row for row in table_rows(out, "## The 20 dearest games")}
        self.assertEqual(rows[str(LO + 1)][1:4], ["same_search", "source", f"board {LO}"])
        self.assertEqual(rows[str(LO + 1)][6], "5,000")

    def test_named_boards(self):
        boards = range(3420, 3431)
        results = [completed(n) for n in boards]
        uppers = [upper(3424), upper(3425, needs=0)] + [upper(n, needs=0) for n in boards if n not in (3424, 3425)]
        argv = ["--results", self.write("r.jsonl", results), "--upper", self.write("u.jsonl", uppers),
                "--era", "3420-3430:1", "--from", "3420"]
        code, out, err = self.run_argv(argv)
        self.assertEqual(code, 0, err)
        section = out.split("## Boards named by the plan")[1].split("## The estimate")[0]
        self.assertIn("- 3424: needs a certificate", section)
        self.assertIn("- 3425: needs no certificate", section)
        self.assertNotIn("3440", section)
        self.assertNotIn("none of", section)

    def test_named_boards_out_of_range(self):
        code, out, _err = self.run_main([completed(LO)], [upper(LO)])
        self.assertEqual(code, 0)
        self.assertIn("- none of 3424, 3425, 3440 is in the range 3300..3300", out)

    def test_estimate(self):
        results = [completed(n, nodes=1000, threads=64, search=100.0) for n in range(LO, LO + 4)]
        uppers = [upper(n) for n in range(LO, LO + 4)]
        code, out, err = self.run_main(results, uppers, eras=[f"{LO}-{LO + 3}:2.0"],
                                       extra=["--price-on-demand", "3.0", "--price-spot", "1.0"])
        self.assertEqual(code, 0, err)
        each = 2.0 * 1000 * (2.0 * 3600 * 64 / 4000) / 3600  # overhead x nodes x c / 3600 = 64 thread-hours
        total = 4 * each
        estimate = out.split("## The estimate")[1]
        self.assertIn(f"- total: {total:,.1f} thread-hours over 4 games that need a certificate", estimate)
        self.assertIn(f"on one machine of 64 threads: {total / 64:,.1f} hours", estimate)
        self.assertIn(f"on demand at $3 an hour: ${total / 64 * 3.0:,.2f}; at spot at $1 an hour: ${total / 64:,.2f}",
                      estimate)
        self.assertIn(f"floor on wall time: {each / 64:,.2f} hours on one machine", estimate)
        recorded = 4 * 2.0 * 100.0 * 64 / 3600
        self.assertIn(f"from the recorded seconds instead of nodes: {recorded:,.1f} thread-hours", estimate)
        for name in ("pilot", "dear", "rest_1", "rest_4"):
            self.assertIn(name, estimate)

    def test_eras_table_shows_the_window_inflation(self):
        results = [completed(n, nodes=1000, threads=64, search=100.0) for n in range(LO, LO + 4)]
        uppers = [upper(n) for n in range(LO, LO + 4)]
        code, out, _err = self.run_main(results, uppers, eras=[f"{LO}-{LO + 3}:2.0"], extra=["--to", str(LO + 3)])
        self.assertEqual(code, 0)
        recorded = 4 * 100.0 * 64 / 3600
        wall = 2.0 * 64
        cells = table_rows(out, "## The eras: what a node costs")[0]
        self.assertEqual(cells[:3], [f"{LO}-{LO + 3}", "4", "2"])
        self.assertEqual(cells[3], f"{wall:,.1f}")
        self.assertEqual(cells[4], f"{recorded:,.1f}")
        self.assertEqual(cells[5], f"{recorded / wall:.2f}")
        self.assertEqual(cells[6], "4,000")
        self.assertEqual(cells[7], f"{2.0 * 3600 * 64 / 4000 * 1e6:,.2f}")

    def test_header_lists_the_inputs_and_the_command(self):
        code, out, _err = self.run_main([completed(LO)], [upper(LO)], gaps=[{"n": LO, "gap": 1}])
        self.assertEqual(code, 0)
        head = out.split("## The eras")[0]
        self.assertIn("- command: `certify_plan.py --results ", head)
        for flag in ("results", "upper", "gaps"):
            self.assertIn(f"- {flag}: {self.dir}", head)
        self.assertIn("- problems: none", head)
        self.assertIn("- machine: 64 threads", head)
        self.assertIn("a certificate costs 2 times the plain search", head)
        self.assertIn("$2.5517 an hour on demand, $0.6752 an hour at spot", head)

    def test_plan_jsonl_has_one_line_per_board_in_order(self):
        results = [completed(n) for n in range(LO, LO + 12)]
        uppers = [upper(n, needs=n % 3 == 0) for n in range(LO, LO + 12)]
        out_dir = self.dir / "out"
        code, out, err = self.run_main(results, uppers, extra=["--out-dir", str(out_dir), "--to", str(LO + 11)])
        self.assertEqual(code, 0, err)
        lines = [json.loads(line) for line in (out_dir / "plan.jsonl").read_text().splitlines()]
        self.assertEqual([g["n"] for g in lines], list(range(LO, LO + 12)))
        self.assertEqual(len({tuple(g) for g in lines}), 1)  # every line has the same fields in the same order
        self.assertEqual((lines[0]["basis"], lines[1]["basis"]), ("chain", "none"))
        self.assertEqual((out_dir / "plan.md").read_text(), out)

    def test_without_an_out_dir_only_the_report_is_printed(self):
        code, out, _err = self.run_main([completed(LO)], [upper(LO)], extra=["--to", str(LO)])
        self.assertEqual(code, 0)
        self.assertTrue(out.startswith("# Certificates campaign plan"))
        self.assertEqual([p.name for p in self.dir.iterdir() if p.is_dir()], [])


class CommandLine(Case):
    def test_runs_as_a_script_from_any_directory(self):
        argv = self.argv([completed(n) for n in range(LO, LO + 4)], [upper(n) for n in range(LO, LO + 4)])
        script = [sys.executable, "-I", str(HERE / "certify_plan.py")]
        done = subprocess.run(script + argv, cwd=self.dir, capture_output=True, text=True)
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIn("## The estimate", done.stdout)
        failed = subprocess.run(script + argv + ["--from", "1"], cwd=self.dir, capture_output=True, text=True)
        self.assertEqual(failed.returncode, 1)
        self.assertEqual(failed.stdout, "")
        self.assertIn("certify_plan.py: error:", failed.stderr)


if __name__ == "__main__":
    unittest.main()
