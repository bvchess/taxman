#!/usr/bin/env python3
"""Tests of the tuning driver (tune.py), with a fake `tune` program in place of the Rust example.

Run from rust/: `python3 -I bench/tuning/test_tune.py`.
"""

import contextlib
import io
import json
import math
import os
import stat
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import make_suite  # noqa: E402
import tune  # noqa: E402

# A stand-in for `tune solve`: the nodes of a problem are 1000 times its number, a configuration changes them by a
# factor named in its tactics (`speed=2` halves them; `slow=on` makes them 20 times as many, past the cutoff; `bug=on`
# changes the best sum), and the CPU seconds are the nodes over 1000.
FAKE = """#!/usr/bin/env python3
import json, sys
argv = sys.argv[1:]
ids = argv[argv.index("--ids") + 1]
tactics = [argv[i + 1] for i, a in enumerate(argv) if a == "--tactic"]
limit = int(argv[argv.index("--node-limit") + 1]) if "--node-limit" in argv else None
number = int(ids.split("-")[-1]) if ids != "root" else 7
nodes = 1000 * number
for t in tactics:
    name, value = t.split("=")
    if name == "speed":
        nodes = nodes // int(value)
    if name == "slow":
        nodes = nodes * 20
completed = limit is None or nodes <= limit
if not completed:
    nodes = limit
best = 500 + (1 if "bug=on" in tactics else 0)
print(json.dumps({"id": ids, "nodes": nodes, "cpu_s": nodes / 1000, "completed": completed, "best": best}))
"""


def make_world(directory, units=9):
    """A suite of `units` units (nodes 1000, 2000, ...) and a whole game, a fake program, and the paths to them."""
    directory = Path(directory)
    suite = directory / "suite.jsonl"
    rows = [{"kind": "game", "game": 3441, "candidates": "f"}]
    for i in range(1, units + 1):
        rows.append({"kind": "unit", "game": 3441, "id": f"3441-{i}", "default": {"nodes": 1000 * i, "cpu_s": i}})
    rows.append({"kind": "whole", "game": 3300, "id": "3300-root", "default": {"nodes": 7000, "cpu_s": 7}})
    suite.write_text("".join(json.dumps(r) + "\n" for r in rows))
    program = directory / "tune"
    program.write_text(FAKE)
    program.chmod(program.stat().st_mode | stat.S_IEXEC)
    return suite, program


def arguments(directory, configs, *extra):
    directory = Path(directory)
    config_file = directory / "configs.json"
    config_file.write_text(json.dumps(configs))
    return [
        "--suite", str(directory / "suite.jsonl"), "--bin", str(directory / "tune"), "--known", "none",
        "--configs", str(config_file), "--results", str(directory / "results.jsonl"), "--workers", "2", *extra,
    ]


def quietly(function, *args):
    """What `function(*args)` prints."""
    return captured(function, *args)[0]


def captured(function, *args):
    """What `function(*args)` prints, and returns."""
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        result = function(*args)
    return out.getvalue(), result


class Statistics(unittest.TestCase):
    def test_geometric_mean(self):
        self.assertAlmostEqual(tune.geomean([1, 4]), 2)
        self.assertTrue(tune.geomean([]) != tune.geomean([]))  # nan

    def test_ratio_survives_zero_time(self):
        self.assertEqual(tune.ratio(0, 0), 1)
        self.assertLess(tune.ratio(0, 5), 1e-3)


class Loading(unittest.TestCase):
    def test_configs_always_have_the_default(self):
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "c.json"
            path.write_text(json.dumps({"lagrange": "lagrange=on", "both": ["lagrange=on", "symmetry=off"]}))
            configs = tune.load_configs(path)
        self.assertEqual(configs, {"default": [], "lagrange": ["lagrange=on"], "both": ["lagrange=on", "symmetry=off"]})

    def test_a_setting_needs_a_value_and_the_default_no_settings(self):
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "c.json"
            for bad in ({"x": ["lagrange"]}, {"default": ["gf2=off"]}):
                path.write_text(json.dumps(bad))
                with self.assertRaises(SystemExit):
                    tune.load_configs(path)

    def test_the_suite_has_problems_not_game_lines(self):
        with tempfile.TemporaryDirectory() as d:
            suite, _ = make_world(d, units=3)
            self.assertEqual([p["id"] for p in tune.load_suite(suite)], ["3441-1", "3441-2", "3441-3", "3300-root"])


class Rungs(unittest.TestCase):
    def test_rungs_are_nested_thirds_of_a_shuffle(self):
        problems = [{"id": str(i)} for i in range(10)]
        sizes, previous = [], set()
        for rung in range(3):
            subset = {p["id"] for p in tune.rung_problems(problems, rung, seed=5)}
            self.assertTrue(previous <= subset)
            previous = subset
            sizes.append(len(subset))
        self.assertEqual(sizes, [4, 7, 10])
        self.assertEqual(tune.rung_problems(problems, 0, 5), tune.rung_problems(problems, 0, 5))

    def test_a_third_of_the_configurations_survive_with_the_default(self):
        problems = [{"id": "a"}, {"id": "b"}]

        def record(cpu):
            return {"ok": True, "result": {"cpu_s": cpu, "nodes": 10, "completed": True, "best": 1}}

        done = {(tune.DEFAULT, p["id"]): record(1.0) for p in problems}
        for name, cpu in (("slow", 3.0), ("fast", 0.5), ("mid", 1.5)):
            done.update({(name, p["id"]): record(cpu) for p in problems})
        done[("unfinished", "a")] = record(0.1)  # quick on the problem it ran, but it did not run the other
        alive = [tune.DEFAULT, "slow", "fast", "mid", "unfinished"]
        # Four configurations besides the default: the best third rounded up is two.
        self.assertEqual(tune.survivors(done, alive, problems), [tune.DEFAULT, "fast", "mid"])


class Running(unittest.TestCase):
    def test_a_run_halves_and_resumes(self):
        configs = {"fast": ["speed=2"], "same": ["speed=1"], "slow": ["slow=on"], "bug": ["bug=on"]}
        with tempfile.TemporaryDirectory() as d:
            make_world(d)
            args = arguments(d, configs)
            text = quietly(tune.main, args)
            results = Path(d) / "results.jsonl"
            first = results.read_text().splitlines()
            rows = [json.loads(line) for line in first]
            # Rung 0: 5 configurations on 4 of the 10 problems; rung 1: the default and the best 2 (fast, and bug, which
            # ties with same and sorts first) on 7, 3 new problems each; rung 2: the default and fast on all 10.
            self.assertEqual(sum(r["rung"] == 0 for r in rows), 5 * 4)
            self.assertEqual(sum(r["rung"] == 1 for r in rows), 3 * 3)
            self.assertEqual(sum(r["rung"] == 2 for r in rows), 2 * 3)
            self.assertIn("!!! BUG? bug", text)
            self.assertIn("<- BEST DIFFERS", text)
            # The slow configuration is stopped at ten times the default's nodes, and counted as cut.
            slow = [r["result"] for r in rows if r["config"] == "slow"]
            self.assertTrue(all(not r["completed"] for r in slow))
            # A second run finds everything done and runs nothing.
            quietly(tune.main, args)
            self.assertEqual(results.read_text().splitlines(), first)

    def test_the_report_ranks_by_cpu_and_finds_a_difference(self):
        with tempfile.TemporaryDirectory() as d:
            make_world(d)
            args = arguments(d, {"fast": ["speed=2"], "bug": ["bug=on"]}, "--rungs", "1")
            quietly(tune.main, args)
            table = quietly(tune.main, [*args, "--report"])
        lines = table.splitlines()
        names = [line.split()[0] for line in lines if line.split() and line.split()[0] in {"fast", "bug", "default"}]
        self.assertEqual(names[0], "fast")
        self.assertIn("0.500", next(line for line in lines if line.startswith("fast")))
        self.assertIn("BEST DIFFERS", next(line for line in lines if line.startswith("bug")))

    def test_a_failed_job_is_recorded_and_run_again(self):
        with tempfile.TemporaryDirectory() as d:
            make_world(d)
            args = arguments(d, {}, "--rungs", "1")
            broken = Path(d) / "tune"
            good = broken.read_text()
            broken.write_text("#!/bin/sh\necho oops >&2\nexit 3\n")
            quietly(tune.main, args)
            rows = [json.loads(line) for line in (Path(d) / "results.jsonl").read_text().splitlines()]
            self.assertTrue(rows and all(not r["ok"] and "oops" in r["error"] for r in rows))
            broken.write_text(good)
            quietly(tune.main, args)
            self.assertEqual(len(tune.read_results(Path(d) / "results.jsonl")), len(rows))

    def test_nice_prefixes_the_command(self):
        with tempfile.TemporaryDirectory() as d:
            make_world(d)
            args = tune.parse(arguments(d, {}, "--nice", "10"))
            problem = tune.load_suite(args.suite)[0]
            command = tune.Runner(args, [problem]).command(["lagrange=on"], problem)
        self.assertEqual(command[:3], ["nice", "-n", "10"])
        self.assertIn("--node-limit", command)
        self.assertEqual(command[command.index("--tactic") + 1], "lagrange=on")



# A stand-in for `tune solve` whose units have the sizes of a table: the nodes of "G-i" are SIZES[G][i].
FAKE_SIZES = """#!/usr/bin/env python3
import json, sys
SIZES = {"3": [150_000, 2_000_000, 50_000, 10 ** 7, 300_000], "4": [10 ** 7] * 4}
argv = sys.argv[1:]
ids = argv[argv.index("--ids") + 1]
limit = int(argv[argv.index("--node-limit") + 1])
game, number = ids.split("-")
nodes = SIZES[game][int(number)]
done = nodes <= limit
print(json.dumps({"id": ids, "game": int(game), "nodes": nodes if done else limit, "cpu_s": 1.0, "wall_s": 1.0,
                  "completed": done, "best": 5, "node_limit": limit}))
"""


def fake_world(directory):
    """The fake `tune solve` in `directory`, and the arguments the prober takes."""
    program = Path(directory) / "tune"
    program.write_text(FAKE_SIZES)
    program.chmod(program.stat().st_mode | stat.S_IEXEC)
    return type("Args", (), {"bin": program, "known": "none", "nice": 0})()


class Suite(unittest.TestCase):
    def test_candidates_are_the_band_in_an_order_that_mixes_its_gaps(self):
        units = [{"id": f"1-{i}", "gap": i} for i in range(100)] + [{"id": "1-none", "gap": None}]
        order = make_suite.candidate_order(units, 10, 49, seed=3)
        self.assertEqual(sorted(u["gap"] for u in order), list(range(10, 50)))
        # Each run of four holds one unit of each fourth of the band.
        self.assertTrue(all(sorted((u["gap"] - 10) // 10 for u in order[i : i + 4]) == [0, 1, 2, 3] for i in (0, 4, 8)))
        self.assertEqual(order, make_suite.candidate_order(units, 10, 49, seed=3))
        self.assertNotEqual(order, make_suite.candidate_order(units, 10, 49, seed=4))

    def test_the_selection_is_log_uniform_and_even_over_the_games(self):
        candidates = [(game, f"{game}-{i}", int(100_000 * 30 ** (i / 39))) for game in (1, 2) for i in range(40)]
        chosen = make_suite.select(candidates, 16, 100_000, 3_000_000)
        self.assertEqual(len(chosen), 16)
        self.assertEqual([sum(c[0] == g for c in chosen) for g in (1, 2)], [8, 8])
        bins = [int(8 * math.log(c[2] / 100_000) / math.log(30)) for c in chosen]
        self.assertEqual([bins.count(b) for b in range(8)], [2] * 8)
        # Where a range has no candidates the others make up for it.
        sparse = [c for c in candidates if c[2] > 400_000]
        self.assertEqual(len(make_suite.select(sparse, 16, 100_000, 3_000_000)), 16)

    def test_a_game_is_probed_until_enough_units_are_in_range(self):
        with tempfile.TemporaryDirectory() as d:
            args = fake_world(d)
            probes = make_suite.Probes(Path(d) / "probes.jsonl")
            prober = make_suite.Prober(args, probes)
            units = [{"game": 3, "id": f"3-{i}"} for i in range(5)]
            # 150,000 nodes (in range) finishes at the 400,000 limit, after failing at the 120,000 it starts with;
            # 2 million finishes at 3 million: two in range, and the third unit is not tried.
            have = captured(prober.game, units, units, "units.jsonl", 2, 100_000, 3_000_000, 120_000)[1]
            self.assertEqual(have, 2)
            self.assertEqual([r["node_limit"] for r in probes.records["3-0"]], [120_000, 400_000])
            self.assertEqual([r["node_limit"] for r in probes.records["3-1"]], [120_000, 400_000, 1_200_000, 3_000_000])
            self.assertEqual(probes.solved("3-1")["nodes"], 2_000_000)
            self.assertEqual(probes.records["3-2"], [])
            # Probed again from a fresh load of the file, it runs nothing.
            before = (Path(d) / "probes.jsonl").read_text()
            again = make_suite.Prober(args, make_suite.Probes(Path(d) / "probes.jsonl"))
            self.assertEqual(captured(again.game, units, units, "units.jsonl", 2, 100_000, 3_000_000, 120_000), ("", 2))
            self.assertEqual((Path(d) / "probes.jsonl").read_text(), before)

    def test_a_unit_that_does_not_finish_within_the_last_limit_is_recorded_and_skipped(self):
        with tempfile.TemporaryDirectory() as d:
            args = fake_world(d)
            probes = make_suite.Probes(Path(d) / "probes.jsonl")
            prober = make_suite.Prober(args, probes)
            units = [{"game": 3, "id": "3-3"}, {"game": 3, "id": "3-4"}]  # 10 million nodes, 300,000
            have = captured(prober.game, units, units, "units.jsonl", 1, 100_000, 3_000_000, 120_000)[1]
            self.assertEqual(have, 1)
            self.assertIsNone(probes.solved("3-3"))
            self.assertEqual(probes.failed_limit("3-3"), 3_000_000)
            self.assertEqual(probes.solved("3-4")["nodes"], 300_000)

    def test_the_defaults_cpu_seconds_are_refreshed_only_for_the_same_trees(self):
        with tempfile.TemporaryDirectory() as d:
            suite, results = Path(d) / "suite.jsonl", Path(d) / "results.jsonl"
            default = {"nodes": 1000, "cpu_s": 2.0, "wall_s": 2.5, "best": 7, "completed": True}
            rows = [{"kind": "suite", "commit": "abc"}, {"kind": "game", "game": 3}]
            rows += [{"kind": "unit", "game": 3, "id": "3-0", "default": default}]
            rows += [{"kind": "whole", "game": 3, "id": "3-root", "default": {**default, "nodes": 5000}}]
            suite.write_text("".join(json.dumps(r) + "\n" for r in rows))

            def record(id, nodes, best=7):
                result = {"nodes": nodes, "cpu_s": 1.5, "wall_s": 1.6, "best": best, "completed": True}
                return {"config": "default", "problem": id, "ok": True, "result": result}

            args = make_suite.parse(["refresh", "--suite", str(suite), "--results", str(results), "--concurrency", "8"])
            results.write_text(json.dumps(record("3-0", 1000)) + "\n" + json.dumps(record("3-root", 5000)) + "\n")
            captured(make_suite.refresh_command, args)
            lines = [json.loads(line) for line in suite.read_text().splitlines()]
            self.assertEqual([lines[2]["default"]["cpu_s"], lines[3]["default"]["cpu_s"]], [1.5, 1.5])
            self.assertEqual((lines[0]["default_cpu_s"], lines[0]["concurrency"], lines[0]["commit"]), (3.0, 8, "abc"))
            # Other trees (the nodes, or the best sum, differ) are refused, and the suite stays as it was.
            before = suite.read_text()
            for bad in (record("3-0", 999), record("3-0", 1000, best=8)):
                results.write_text(json.dumps(bad) + "\n" + json.dumps(record("3-root", 5000)) + "\n")
                with self.assertRaises(SystemExit):
                    captured(make_suite.refresh_command, args)
            self.assertEqual(suite.read_text(), before)
            # With --new-trees other nodes are taken (and recorded), but not another best sum.
            args = make_suite.parse(["refresh", "--suite", str(suite), "--results", str(results), "--concurrency", "8",
                                     "--new-trees", "--commit", "def"])
            results.write_text(json.dumps(record("3-0", 600)) + "\n" + json.dumps(record("3-root", 4000)) + "\n")
            captured(make_suite.refresh_command, args)
            lines = [json.loads(line) for line in suite.read_text().splitlines()]
            self.assertEqual([lines[2]["default"]["nodes"], lines[3]["default"]["nodes"]], [600, 4000])
            self.assertEqual((lines[0]["commit"], lines[2]["default"]["best"]), ("def", 7))
            before = suite.read_text()
            wrong = json.dumps(record("3-0", 600, best=8))
            results.write_text(wrong + "\n" + json.dumps(record("3-root", 4000)) + "\n")
            with self.assertRaises(SystemExit):
                captured(make_suite.refresh_command, args)
            self.assertEqual(suite.read_text(), before)

    def test_a_game_is_given_up_after_units_that_do_not_finish(self):
        with tempfile.TemporaryDirectory() as d:
            args = fake_world(d)
            probes = make_suite.Probes(Path(d) / "probes.jsonl")
            prober = make_suite.Prober(args, probes)
            units = [{"game": 4, "id": f"4-{i}"} for i in range(4)]
            have = captured(prober.game, units, units, "units.jsonl", 5, 100_000, 3_000_000, 120_000, 2)[1]
            self.assertEqual(have, 0)
            self.assertEqual([probes.failed_limit(f"4-{i}") for i in range(4)], [3_000_000, 3_000_000, 0, 0])
            # Tried again, the two that failed are not tried again (nor do they count against the game).
            again = captured(prober.game, units, units, "units.jsonl", 5, 100_000, 3_000_000, 120_000, 2)[1]
            self.assertEqual(again, 0)
            self.assertEqual([probes.failed_limit(f"4-{i}") for i in range(4)], [3_000_000] * 4)


if __name__ == "__main__":
    unittest.main()
