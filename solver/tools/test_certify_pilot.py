#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Unit tests of certify_pilot.py: on synthetic certificate lines, on real ones (solver/tools/certify/
certificates_sample_1301_2175.jsonl, made by `taxman certify` on the laptop) and on the committed plan and pilot list.

usage: python3 -I test_certify_pilot.py      (or python3 -I -m unittest test_certify_pilot, from this directory)
"""
import contextlib
import io
import json
import statistics
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))  # python -I puts neither the script's directory nor the current one on the path

import certify_campaign  # noqa: E402
import certify_pilot  # noqa: E402

CERTIFY = HERE / "certify"
PLAN_JSONL = CERTIFY / "plan.jsonl"
PLAN_MD = CERTIFY / "plan.md"
PROBLEMS = CERTIFY / "problems_600_3440.jsonl"
PILOT_LIST = CERTIFY / "lists/stage4/pilot7.txt"
SAMPLE = CERTIFY / "certificates_sample_1301_2175.jsonl"

C = 100.0  # microseconds a node, in the synthetic plans
NODES = 1_000_000  # the chain's nodes of a synthetic game: N c is 100 thread-seconds
THREADS = 10


def plan_md(eras=((1000, 1999, C), (2000, 2999, 2 * C)), threads=64, on_demand=2.5, spot=0.5):
    """The text of a plan.md with the eras table and the lines the report reads."""
    rows = "\n".join(f"| {lo}-{hi} | 1,000 | 6.4 | 400.0 | 1,300.0 | 3.25 | 6,000,000,000 | {c:,.2f} |"
                     for lo, hi, c in eras)
    return (f"# Certificates campaign plan\n\n- machine: {threads} threads\n- overhead: a certificate costs 2 times\n"
            f"- prices: ${on_demand} an hour on demand, ${spot} an hour at spot\n\n## The eras: what a node costs\n\n"
            "The chain ran several searches at once.\n\n| era | games | wall hours | wall thread-hours | recorded "
            "thread-hours | recorded / wall | nodes | c (us per node) |\n|---|---:|---:|---:|---:|---:|---:|---:|\n"
            f"{rows}\n\nA repeat whose source was proven at its ceiling.\n\n## By band of 100 boards\n\n| a | b |\n")


def plan_line(n, nodes=NODES, basis="chain", needs=1, share=None):
    """A line of plan.jsonl: the plan prices the certificate at twice the plain search, N c thread-seconds."""
    return {"n": n, "needs": needs, "share": share, "basis": basis, "cost_nodes": nodes,
            "expected_thread_hours": 2 * nodes * C / 1e6 / 3600}


def cert(n, r=2.0, r_nodes=1.0, verdict="certified", threads=THREADS, members=3, same=None, nodes=NODES, c=C,
         extra=None):
    """A certificate line of a game of `nodes` chain nodes at the cost c, searched with R = r and R_nodes = r_nodes."""
    thread_seconds = r * nodes * c / 1e6
    row = {"n": n, "score": 10 * n, "verdict": verdict, "open": list(range(members)), "rejected": 0, "frames": 7,
           "two_sided": 300, "one_sided": members, "in_the_bag": 5, "nodes": round(r_nodes * nodes),
           "seconds": thread_seconds / threads, "seconds_all_runs": thread_seconds / threads, "threads": threads,
           "width": 16, "digest": "00000000000000ff"}
    if same is not None:
        same_problem = {"n": same, "digest": row["digest"], "best": 1, "worth": 1}
        row.update(nodes=0, seconds=0.0, seconds_all_runs=0.0, same_problem=same_problem)
    if verdict == "beaten":
        row["beaten_by"] = {"score": 10 * n + 1, "moves": [2, 3]}
    row.update(extra or {})
    return row


class Case(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)

    def write(self, name, rows):
        path = self.dir / name
        path.write_text("".join((row if isinstance(row, str) else json.dumps(row)) + "\n" for row in rows))
        return str(path)

    def write_md(self, text=None):
        path = self.dir / "plan.md"
        path.write_text(plan_md() if text is None else text)
        return str(path)

    @staticmethod
    def call(argv):
        """(exit status, stdout, stderr) of the tool's main on the arguments."""
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = certify_pilot.main([str(arg) for arg in argv])
        return code, out.getvalue(), err.getvalue()

    def run_tool(self, certs, boards, listed=None, plan_rows=None, md=None, extra=()):
        """The tool on certificate lines `certs` against a synthetic plan, bounds and problems for the `boards`."""
        plan_rows = plan_rows if plan_rows is not None else [plan_line(n) for n in boards]
        argv = [self.write("certs.jsonl", certs), "--plan", self.write("plan.jsonl", plan_rows),
                "--plan-md", self.write_md(md),
                "--upper", self.write("upper.jsonl", [{"n": n, "gap": 700 + n} for n in boards]),
                "--problems", self.write("problems.jsonl", [{"n": n, "words": 2} for n in boards])]
        if listed is not None:
            argv += ["--list", self.write("list.txt", ["# a list"] + [str(n) for n in listed])]
        return self.call(argv + list(extra))


class Rates(Case):
    def test_r_is_the_thread_seconds_over_the_chains_nodes_times_the_cost_per_node(self):
        certs = [cert(1100, r=2.5, r_nodes=1.5), cert(1200, r=1.2, r_nodes=0.8, threads=64)]
        code, out, err = self.run_tool(certs, [1100, 1200])
        self.assertEqual((code, err), (0, ""))
        row = {line.split()[0]: line.split() for line in out.splitlines() if line[:4] in ("1100", "1200")}
        # n, |O|, gap, words, width, nodes, chain nodes, R_nodes, R_node, R, thread-hours, plan hours, basis
        self.assertEqual(row["1100"][:7], ["1100", "3", "1800", "2", "16", "1,500,000", "1,000,000"])
        self.assertEqual(row["1100"][7:10], ["1.50", "1.67", "2.50"])
        self.assertEqual(row["1200"][7:10], ["0.80", "1.50", "1.20"])
        self.assertIn("R over 2 rated games: median 1.85, largest 2.50 (board 1100), smallest 1.20; the plan priced 2",
                      out)

    def test_r_equals_r_nodes_times_r_node(self):
        game = certify_pilot.Searched(1100, 3, 16, 1_500_000, 250.0, NODES, C, "chain", 0.0, None, None)
        self.assertAlmostEqual(game.r_nodes, 1.5)
        self.assertAlmostEqual(game.r, 2.5)
        self.assertAlmostEqual(game.r_node, 250.0 * 1e6 / 1_500_000 / C)
        self.assertAlmostEqual(game.r, game.r_nodes * game.r_node)

    def test_a_later_era_has_its_own_cost_per_node(self):
        # 2100 is in the era of c = 200: thread-seconds that make R = 2 at c = 100 make R = 1 there.
        code, out, _err = self.run_tool([cert(2100, r=2.0, c=2 * C)], [2100])
        self.assertEqual(code, 0)
        self.assertIn("R over 1 rated games: median 2.00", out)
        code, out, _err = self.run_tool([cert(2100, r=2.0, c=C)], [2100])
        self.assertIn("R over 1 rated games: median 1.00", out)

    def test_a_game_that_was_stopped_and_resumed_is_rated_on_the_final_line(self):
        stopped = cert(1100, r=1.0, verdict="incomplete", extra={"nodes": 400_000})
        final = cert(1100, r=3.0, r_nodes=1.2)
        code, out, _err = self.run_tool([stopped, final], [1100])
        self.assertEqual(code, 0)
        self.assertIn("R over 1 rated games: median 3.00", out)
        self.assertIn("1 stopped once or more and resumed", out)

    def test_the_games_the_rule_settled_are_counted_and_not_rated(self):
        certs = [cert(1100, r=2.0), cert(1101, same=1100), cert(1102, same=1101)]
        code, out, _err = self.run_tool(certs, [1100, 1101, 1102])
        self.assertEqual(code, 0)
        self.assertIn("1 games searched, 2 settled by the rule of the same problem, 0 beaten", out)
        self.assertIn("R over 1 rated games: median 2.00", out)

    def test_a_game_priced_by_the_cost_model_is_shown_and_not_rated(self):
        plan = [plan_line(1100), plan_line(1101, basis="gap model")]
        code, out, _err = self.run_tool([cert(1100, r=2.0), cert(1101, r=9.0)], [1100, 1101], plan_rows=plan)
        self.assertEqual(code, 0, "an R of 9 on a game the plan priced by the model does not stop the campaign")
        self.assertIn("gap model (not rated)", out)
        self.assertIn("R over 1 rated games", out)

    def test_the_search_time_is_given_against_the_plans_and_in_dollars(self):
        # R = 2 and 4: 200 and 400 thread-seconds, against the plan's 200 for each.
        _code, out, _err = self.run_tool([cert(1100, r=2.0), cert(1200, r=4.0)], [1100, 1200])
        hours = 600 / 3600
        self.assertIn(f"search time: {hours:,.1f} thread-hours against the plan's {400 / 3600:,.1f} for the same games "
                      "(1.50 times)", out)
        machine = hours / 64
        self.assertIn(f"on 64 threads that is {machine:,.2f} hours, ${machine * 0.5:,.2f} at spot or "
                      f"${machine * 2.5:,.2f} on demand", out)

    def test_bands_give_the_median_r_by_gap_and_by_open_members(self):
        certs = [cert(1100, r=2.0, members=10), cert(1150, r=3.0, members=60), cert(1200, r=4.0, members=61)]
        code, out, _err = self.run_tool(certs, [1100, 1150, 1200], extra=["--bands"])
        self.assertEqual(code, 0)
        lines = [line.split() for line in out.splitlines()]
        # The synthetic gaps are 700 + n: 1800, 1850 and 1900.
        self.assertIn(["1800-1899", "2", "2.50", "3.00"], lines)
        self.assertIn(["1900-1999", "1", "4.00", "4.00"], lines)
        self.assertIn(["0-49", "1", "2.00", "2.00"], lines)
        self.assertIn(["50-99", "2", "3.50", "4.00"], lines)


class Rule(Case):
    boards = [1100, 1101, 1102, 1103, 1104, 1105, 1106]

    def games(self, rates):
        return [cert(n, r=r) for n, r in zip(self.boards, rates)]

    def test_go_if_the_median_is_at_most_3_and_no_game_above_5(self):
        code, out, _err = self.run_tool(self.games([1.5, 2, 2.5, 3.0, 3.0, 4.9, 2]), self.boards, listed=self.boards)
        self.assertEqual(code, 0)
        self.assertIn("stop rule: GO: the median R is 2.50, at most 3, and the largest 4.90, at most 5", out)

    def test_stop_if_the_median_is_above_3(self):
        code, out, _err = self.run_tool(self.games([3.5, 3.5, 3.5, 3.5, 1, 1, 1]), self.boards, listed=self.boards)
        self.assertEqual(code, 3)
        self.assertIn("stop rule: STOP: the median R is 3.50, above 3", out)

    def test_stop_if_one_game_is_above_5(self):
        code, out, _err = self.run_tool(self.games([1, 1, 1, 1, 1, 1, 5.01]), self.boards, listed=self.boards)
        self.assertEqual(code, 3)
        self.assertIn("stop rule: STOP: board 1106 has R 5.01, above 5", out)

    def test_exactly_3_and_exactly_5_pass(self):
        code, out, _err = self.run_tool(self.games([3, 3, 3, 3, 3, 3, 5]), self.boards, listed=self.boards)
        self.assertEqual(code, 0, out)

    def test_stop_at_once_if_a_game_is_above_5_though_games_are_missing(self):
        code, out, _err = self.run_tool(self.games([6.0]), self.boards, listed=self.boards)
        self.assertEqual(code, 3)
        self.assertIn("stop rule: STOP: board 1100 has R 6.00", out)

    def test_unfinished_while_games_of_the_list_have_no_verdict(self):
        code, out, _err = self.run_tool(self.games([2, 2, 2]), self.boards, listed=self.boards)
        self.assertEqual(code, 4)
        self.assertIn("stop rule: UNFINISHED: 4 games of the list have no verdict yet: 1103, 1104, 1105, 1106", out)

    def test_a_game_with_only_an_incomplete_line_has_no_verdict_yet(self):
        certs = self.games([2, 2, 2, 2, 2, 2]) + [cert(1106, verdict="incomplete")]
        code, out, _err = self.run_tool(certs, self.boards, listed=self.boards)
        self.assertEqual(code, 4)
        self.assertIn("1 games of the list have no verdict yet: 1106", out)

    def test_a_beaten_line_is_a_counterexample_whatever_else_there_is(self):
        certs = self.games([2, 2, 2, 2, 2, 2]) + [cert(1106, verdict="beaten")]
        code, out, _err = self.run_tool(certs, self.boards, listed=self.boards)
        self.assertEqual(code, 2)
        self.assertIn("stop rule: COUNTEREXAMPLE: a game beats its recorded score: board 1106", out)
        code, out, _err = self.run_tool([cert(1100, verdict="beaten")], self.boards, listed=self.boards)
        self.assertEqual(code, 2, "also with the other games missing")

    def test_without_a_list_the_rule_covers_the_games_given(self):
        code, out, _err = self.run_tool(self.games([2, 2, 4]), self.boards[:3])
        self.assertEqual(code, 0)
        self.assertIn("median R is 2.00", out)
        code, out, _err = self.run_tool([cert(1100, same=1099)], [1100])
        self.assertEqual(code, 4)
        self.assertIn("UNFINISHED: no searched game to rate", out)

    def test_games_with_a_verdict_outside_the_list_are_noted_and_not_waited_for(self):
        code, out, _err = self.run_tool(self.games([2, 2, 2]), self.boards, listed=self.boards[:2])
        self.assertEqual(code, 0)
        self.assertIn("note: 1 games with a verdict are not in the list: 1102", out)


class Inputs(Case):
    def test_the_last_final_line_of_a_board_decides(self):
        code, out, _err = self.run_tool([cert(1100, r=4.0), cert(1100, r=2.0)], [1100])
        self.assertEqual(code, 0)
        self.assertIn("median 2.00", out)

    def test_errors_name_the_file_and_the_line(self):
        cases = [({"n": 1100}, "certs.jsonl line 1 (board 1100): no field 'verdict'"),
                 (dict(cert(1100), verdict="maybe"), "unknown verdict 'maybe'"),
                 (dict(cert(1100), nodes="x"), "field 'nodes' is 'x', not an integer"),
                 (dict(cert(1100), seconds_all_runs=-1), "field 'seconds_all_runs' is -1, below 0"),
                 ("not json", "certs.jsonl line 1: not JSON")]
        for row, message in cases:
            code, _out, err = self.run_tool([row], [1100])
            self.assertEqual(code, 1, message)
            self.assertIn(message, err)

    def test_a_board_the_plan_does_not_know_is_an_error(self):
        code, _out, err = self.run_tool([cert(1100)], [1100], plan_rows=[plan_line(1101)])
        self.assertEqual(code, 1)
        self.assertIn("board 1100: plan.jsonl has no line for it", err)
        _code, _out, err = self.run_tool([cert(1100)], [1100], plan_rows=[plan_line(1100, needs=0)])
        self.assertIn("board 1100: plan.jsonl says it needs no certificate", err)
        _code, _out, err = self.run_tool([cert(5000)], [5000])
        self.assertIn("board 5000: no era of plan.md holds it", err)

    def test_a_plan_md_without_its_table_is_an_error(self):
        code, _out, err = self.run_tool([cert(1100)], [1100], md="# nothing\n")
        self.assertEqual(code, 1)
        self.assertIn("no section 'The eras: what a node costs'", err)
        _code, _out, err = self.run_tool([cert(1100)], [1100], md=plan_md().replace("- machine: 64 threads\n", ""))
        self.assertIn("cannot read the eras, the machine and the prices from it", err)

    def test_a_file_that_is_not_there(self):
        code, _out, err = self.call([self.dir / "none.jsonl"])
        self.assertEqual(code, 1)
        self.assertIn("none.jsonl: cannot read", err)

    def test_the_list_must_be_a_list(self):
        argv = [self.write("certs.jsonl", [cert(1100)]), "--list", self.write("bad.txt", ["1100", "x"]),
                "--plan", self.write("plan.jsonl", [plan_line(1100)]), "--plan-md", self.write_md()]
        code, _out, err = self.call(argv)
        self.assertEqual(code, 1)
        self.assertIn("bad.txt line 2: expected a board size", err)


class Committed(unittest.TestCase):
    """The tool on the plan, the pilot list and the data as committed."""

    heads = [n for n in certify_campaign.read_list(PILOT_LIST) if n not in (3337, 3409, 3421)]

    def test_plan_md_has_the_two_eras_the_stop_rule_names(self):
        plan = certify_pilot.read_plan_md(PLAN_MD)
        self.assertEqual([(e.lo, e.hi, e.c) for e in plan.eras], [(2176, 3310, 243.84), (3311, 3440, 326.73)])
        self.assertEqual((plan.machine_threads, plan.on_demand, plan.spot), (64, 2.5517, 0.6752))

    @staticmethod
    def pilot_certificates(rates):
        """Certificate lines of the pilot list: R = rates[n] for each head (R_nodes 1.3), the groups settled by the
        rule, with the open members and the set width of the committed problems."""
        plan = certify_pilot.read_plan_md(PLAN_MD)
        rows = {json.loads(line)["n"]: json.loads(line) for line in PLAN_JSONL.read_text().splitlines()}
        problems = {json.loads(line)["n"]: json.loads(line) for line in PROBLEMS.read_text().splitlines()}
        lines = []
        for n in certify_campaign.read_list(PILOT_LIST):
            era = next(e for e in plan.eras if e.lo <= n <= e.hi)
            if rows[n]["share"] is not None:
                lines.append(cert(n, same=rows[n]["share"]))
            else:
                lines.append(cert(n, r=rates[n], r_nodes=1.3, threads=64, nodes=rows[n]["chain_nodes"], c=era.c,
                                  members=rows[n]["open"], extra={"width": problems[n]["width"]}))
        return lines

    @staticmethod
    def run_pilot(lines):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "pilot7.jsonl"
            path.write_text("".join(json.dumps(line) + "\n" for line in lines))
            return Case.call([path, "--list", PILOT_LIST])

    def test_the_pilot_at_the_plans_price_passes_with_r_2_everywhere(self):
        code, out, err = self.run_pilot(self.pilot_certificates({n: 2.0 for n in self.heads}))
        self.assertEqual((code, err), (0, ""))
        self.assertIn("7 games searched, 3 settled by the rule of the same problem, 0 beaten", out)
        self.assertIn("R over 7 rated games: median 2.00, largest 2.00", out)
        self.assertIn("R_nodes: median 1.30; R_node: median 1.54", out)
        self.assertIn("search time: 288.0 thread-hours against the plan's 288.0 for the same games (1.00 times)", out)
        self.assertIn("stop rule: GO", out)
        # |O|, the root gap (the barred bounds) and the GF(2) words and set width (the problems file) of two games.
        rows = {line.split()[0]: line.split() for line in out.splitlines() if line.split()[:1] in (["3175"], ["3300"])}
        self.assertEqual(rows["3175"][1:5], ["255", "1388", "4", "128"])
        self.assertEqual(rows["3300"][1:3], ["246", "1443"])

    def test_the_pilot_stops_on_a_median_above_3_and_on_a_game_above_5(self):
        code, out, _err = self.run_pilot(self.pilot_certificates({n: 3.2 for n in self.heads}))
        self.assertEqual(code, 3)
        self.assertIn("STOP: the median R is 3.20, above 3", out)
        spike = {n: 2.0 for n in self.heads}
        spike[3300] = 5.5
        code, out, _err = self.run_pilot(self.pilot_certificates(spike))
        self.assertEqual(code, 3)
        self.assertIn("STOP: board 3300 has R 5.50, above 5", out)

    def test_the_pilot_after_its_first_game_is_unfinished(self):
        lines = self.pilot_certificates({n: 2.0 for n in self.heads})[:1]
        code, out, _err = self.run_pilot(lines)
        self.assertEqual(code, 4)
        self.assertIn("UNFINISHED: 9 games of the list have no verdict yet: 2340, 3420, 3421, 3336, 3337, 3335, 3175, "
                      "3408, ...", out)

    def test_the_pilot_with_a_counterexample_stops_everything(self):
        lines = self.pilot_certificates({n: 2.0 for n in self.heads})
        lines[0] = dict(lines[0], verdict="beaten", beaten_by={"score": 1, "moves": [2]})
        code, out, _err = self.run_pilot(lines)
        self.assertEqual(code, 2)
        self.assertIn("COUNTEREXAMPLE: a game beats its recorded score: board 3300", out)


class Real(unittest.TestCase):
    """Certificate lines that `taxman certify` wrote on the laptop for games of 1301-2175: eight searches, three games
    settled by the rule, and one search that a node limit stopped and a second run finished."""

    def setUp(self):
        self.rows = [json.loads(line) for line in SAMPLE.read_text().splitlines()]
        self.assertEqual(len(self.rows), 12)
        self.final = {row["n"]: row for row in self.rows if row["verdict"] != "incomplete"}

    def run_sample(self, c=100.0, extra=()):
        """The tool on the real lines against a plan with twice each game's nodes as the chain's, at the cost c."""
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            plan = [{"n": n, "needs": 1, "share": None, "basis": "chain", "cost_nodes": 2 * max(row["nodes"], 1),
                     "expected_thread_hours": 1.0} for n, row in self.final.items()]
            (tmp / "plan.jsonl").write_text("".join(json.dumps(line) + "\n" for line in plan))
            (tmp / "plan.md").write_text(plan_md(eras=[(1301, 2175, c)]))
            return Case.call([SAMPLE, "--plan", tmp / "plan.jsonl", "--plan-md", tmp / "plan.md", *extra])

    def test_the_real_lines_are_read_and_rated(self):
        code, out, err = self.run_sample()
        self.assertEqual((code, err), (0, ""))
        self.assertIn("8 games searched, 3 settled by the rule of the same problem, 0 beaten; 1 stopped once or more "
                      "and resumed", out)
        searched = [row for row in self.final.values() if "same_problem" not in row]
        expected = [row["seconds_all_runs"] * row["threads"] * 1e6 / (2 * row["nodes"] * 100.0) for row in searched]
        self.assertIn(f"R over 8 rated games: median {statistics.median(expected):.2f}, largest {max(expected):.2f}",
                      out)
        self.assertIn("R_nodes: median 0.50", out)  # the plan holds twice the nodes of every game
        # The resumed game is rated on its final line: the nodes and the seconds of all its runs.
        line = next(line for line in out.splitlines() if line.startswith("2128"))
        self.assertIn("6,862", line)
        self.assertNotIn("3,954", line)
        self.assertGreater(self.final[2128]["seconds_all_runs"], self.final[2128]["seconds"])

    def test_the_real_lines_stop_at_a_cost_per_node_that_is_too_small_and_pass_at_a_large_one(self):
        code, out, _err = self.run_sample(c=1.0)  # R is a hundred times what it is at c = 100
        self.assertEqual(code, 3)
        self.assertIn("above 5", out)
        code, out, _err = self.run_sample(c=1e6)
        self.assertEqual(code, 0)
        self.assertIn("stop rule: GO", out)

    def test_the_real_lines_with_a_list_wait_for_the_games_of_the_list(self):
        with tempfile.TemporaryDirectory() as tmp:
            listed = Path(tmp) / "list.txt"
            listed.write_text("1312\n1313\n1400\n")
            code, out, _err = self.run_sample(extra=["--list", listed])
        self.assertEqual(code, 4)
        self.assertIn("UNFINISHED: 1 games of the list have no verdict yet: 1400", out)
        self.assertIn("note: 9 games with a verdict are not in the list: 1314, 1315, 1316, 1317, 1650, 1782, 2107, "
                      "2109", out)


if __name__ == "__main__":
    unittest.main()
