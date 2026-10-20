#!/usr/bin/env python3
"""Tests of analyze.py on lines made up to have known answers.

Run it as `python3 -I test_analyze.py`.
"""

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import analyze  # noqa: E402


def fit(lows, total):
    """A fit with these lowest-so-far bounds, whose k-th step ends k/len of the way through `total` seconds."""
    return {"live": 5, "steps": len(lows), "lows": lows, "ts": [total * (k + 1) / len(lows) for k in range(len(lows))],
            "s": total}


def line(id_, gap, subtree, lows, solve_s, n=3300, source="suite", sub=None):
    """A node needing 85: its fit's `lows` cut it if one is below that."""
    return {"n": n, "id": id_, "source": source, "gap": gap, "need": 85, "subtree": subtree, "completed": True,
            "solve_cpu_s": solve_s, "today_s": 0.001, "graph_s": 0.01, "strong": fit(lows, 0.5), "sub": sub}


def a1(r2, full, build_s=0.001):
    """The A1 bounds of a node needing 85 with today's bound `gap + 85`: each given as (bound, CPU s, capped)."""
    one = lambda bound, s, capped=False: {"bound": bound, "cut": bound < 85, "capped": capped, "s": s}  # noqa: E731
    return {"build_s": build_s, "r2": one(*r2), "full": one(*full)}


class Analysis(unittest.TestCase):
    def test_the_a1_bounds_are_two_more_variants_that_cost_their_matroids_and_their_seconds(self):
        # Gap 100: today's bound 185, so a bound of 85 closes 100 of the 101 above a cut.
        cut = dict(line("a", 100, 1000, [100], 1.0), today=185, a1=a1((80, 0.02), (84, 1.5)))
        capped = dict(line("b", 100, 1000, [100], 1.0), today=185, a1=a1((130, 0.02), (95, 30.0, True)))
        without = dict(line("c", 100, 1000, [100], 1.0), today=185)
        nodes = analyze.nodes_of([cut, capped, without], 99)
        self.assertEqual(len(nodes), 3)
        self.assertEqual(nodes[0]["outcomes"]["a1r2"], (True, 0.021))
        self.assertEqual(nodes[1]["outcomes"]["a1full"], (False, 30.001))
        self.assertNotIn("a1r2", nodes[2]["outcomes"])
        # The share cut counts the nodes that have the variant only; a policy applies it to those only.
        self.assertEqual(analyze.share(nodes, "a1r2"), 0.5)
        self.assertEqual(analyze.share(nodes, "a1full"), 0.5)
        applied, cuts, net = analyze.evaluate(nodes, "a1r2", analyze.gap_above(0, (0.0, 0.01, 1.0)))
        self.assertEqual((applied, cuts), (2, 1))
        # Searching the cut node costs 1000 steps at 1 ms; both nodes pay 0.021 s for the bound.
        self.assertAlmostEqual(net, 1.0 - 2 * 0.021)
        # How much of the excess over a cut a bound closes: all of it when it cuts, a part otherwise.
        self.assertEqual(analyze.closed(nodes[0], "full"), 1.0)
        self.assertAlmostEqual(analyze.closed(nodes[1], "full"), 90 / 101)
        self.assertEqual(analyze.closed(nodes[1], "r2"), 55 / 101)

    def test_the_cost_of_a_fit_is_the_matroids_and_the_steps_up_to_the_cut(self):
        node = line("a", 100, 10, [100, 90, 80, 70, 60], 0.0)
        cut, cost = analyze.outcome(node, node["strong"], 99)
        self.assertTrue(cut)
        self.assertAlmostEqual(cost, 0.01 + 0.5 * 3 / 5)
        cut, cost = analyze.outcome(node, node["strong"], 2)
        self.assertFalse(cut)
        self.assertAlmostEqual(cost, 0.01 + 0.5 * 2 / 5)
        # The first step alone is the root's multipliers kept.
        self.assertEqual(analyze.outcome(node, node["strong"], 1)[0], False)

    def test_the_best_policy_leaves_out_the_nodes_whose_search_is_cheaper_than_the_bound(self):
        # 1 ms a step. The nodes of gap 100 (10 steps) and 200 (1000 steps) and 300 (100000) are all cut by the fit,
        # which costs 0.01 + 0.5 s, and the node of gap 150 is not cut.
        cheap, middle, dear = [100, 90, 80], [100, 90, 80], [100, 90, 80]
        lines = [
            line("a", 100, 10, cheap, 0.01),
            line("b", 150, 20, [100, 95, 90], 0.02),
            line("c", 200, 1000, middle, 1.0),
            line("d", 300, 100000, dear, 100.0),
        ]
        nodes = analyze.nodes_of(lines, 99)
        self.assertEqual(len(nodes), 4)
        model = (0.0, 0.01, 1.0)  # log10 subtree = gap / 100
        # Everywhere: the three cuts save 0.01 + 1 + 100 seconds, the four fits cost 4 * 0.51 (each takes its three
        # steps: b is not cut, and the others are cut by the third).
        applied, cut, net = analyze.evaluate(nodes, "strong", analyze.gap_above(0, model))
        self.assertEqual((applied, cut), (4, 3))
        self.assertAlmostEqual(net, 0.01 + 1.0 + 100.0 - 4 * 0.51)
        # Above a forecast of 10^2.5 steps (gap 250): only d, 100 s saved for 0.51 spent.
        floor = analyze.gap_above(10 ** 2.5, model)
        self.assertAlmostEqual(floor, 250)
        applied, cut, net = analyze.evaluate(nodes, "strong", floor)
        self.assertEqual((applied, cut), (1, 1))
        self.assertAlmostEqual(net, 100.0 - 0.51)
        # A node whose fit is not recorded for a variant is not counted for it.
        self.assertEqual(analyze.evaluate(nodes, "sub", analyze.gap_above(0, model))[:2], (0, 0))

    def test_the_forecast_and_the_rate_follow_the_completed_solves(self):
        # A decade of steps for each 100 of gap, from 1000 at gap 0, at 2 ms a step.
        steps = [round(10 ** (3 + gap / 100)) for gap in range(0, 500, 100)]
        lines = [line(str(k), 100 * k, subtree, [100], 0.002 * subtree) for k, subtree in enumerate(steps)]
        nodes = analyze.nodes_of(lines, 99)
        self.assertAlmostEqual(nodes[0]["search_s"] / nodes[0]["subtree"], 0.002)
        a, b, r2 = analyze.forecast(nodes, "fit")
        self.assertAlmostEqual((a, b, r2), (3.0, 0.01, 1.0), places=3)
        a, b, _ = analyze.forecast(nodes, "floor")
        self.assertEqual(b, analyze.FLOOR_SLOPE)
        self.assertAlmostEqual(a, 3 + 200 * (0.01 - analyze.FLOOR_SLOPE), places=3)

    def test_paths_closed_nodes_and_repeats_are_left_out_and_cut_lines_are_ignored(self):
        good = line("a", 10, 5, [100], 0.0)
        lines = [good, dict(good, id="b", source="path"), dict(good, id="c", gap=-1), dict(good, id="d", subtree=None)]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "worker-0.jsonl"
            path.write_text("\n".join(json.dumps(x) for x in lines + [good]) + '\n{"id": "e", "n"')
            read = analyze.read([directory])
        self.assertEqual([x["id"] for x in read], ["a", "b", "c", "d"])
        self.assertEqual([n["id"] for n in analyze.nodes_of(read, 99)], ["a"])


if __name__ == "__main__":
    unittest.main()
