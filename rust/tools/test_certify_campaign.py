#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Unit tests of certify_campaign.py, on synthetic work lists, and its lists as committed (bench/certify/lists/stage4).

usage: python3 -I test_certify_campaign.py      (or python3 -I -m unittest test_certify_campaign, from this directory)
"""
import contextlib
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))  # python -I puts neither the script's directory nor the current one on the path

import certify_campaign  # noqa: E402

CRATE = HERE.parent
PLAN = CRATE / "bench/certify/plan.jsonl"
STAGE4 = CRATE / "bench/certify/lists/stage4"
PILOT = certify_campaign.PILOT_HEADS
NAMES = ["dear_rest", "deferred", "pilot7", "rest_1", "rest_2", "rest_3", "rest_4"]


def game(n, hours=1.0, share=None, proof="exhausted", needs=1):
    """A line of the work list with the fields the lists use."""
    return {"n": n, "needs": needs, "share": share, "proof": proof if needs else None,
            "expected_thread_hours": hours if needs else 0.0}


def synthetic():
    """A work list with every kind of game the campaign tells apart (43 games that need a certificate).

    The seven pilot games cost 10 + k hours (k = 0..6 in the order of PILOT); 3500 and 3501 share with the pilot game
    3300, and 3510 with 3500.  The heads 3600 + k (k = 0..21) cost 100 - k, the dearest of the plan, and 3601 repeats a
    search with no share.  The heads 3700..3707 cost 1.0 to 1.7, and 3710 shares with 3700.  3720 repeats a search with
    no share (2 hours) and 3721 shares with it.  3730 needs no certificate.
    """
    games = [game(n, hours=10.0 + k) for k, n in enumerate(PILOT)]
    games += [game(3500, share=3300, proof="same_search", hours=0.0)]
    games += [game(3501, share=3300, proof="same_search", hours=0.0)]
    games += [game(3510, share=3500, proof="same_search", hours=0.0)]
    games += [game(3600 + k, hours=100.0 - k, proof="same_search" if k == 1 else "exhausted") for k in range(22)]
    games += [game(3700 + k, hours=1.0 + k / 10) for k in range(8)]
    games += [game(3710, share=3700, proof="same_search", hours=0.0)]
    games += [game(3720, proof="same_search", hours=2.0), game(3721, share=3720, proof="same_search", hours=0.0)]
    games += [game(3730, needs=0)]
    return sorted(games, key=lambda g: g["n"])


class Lists(unittest.TestCase):
    def setUp(self):
        self.games = synthetic()
        self.made = certify_campaign.campaign_lists(self.games, 2)
        self.lists = {entry["name"]: entry for entry in self.made}

    def test_the_lists_are_in_the_order_they_run(self):
        names = [entry["name"] for entry in self.made]
        self.assertEqual(names, ["pilot7", "dear_rest", "rest_1", "rest_2", "deferred"])
        self.assertEqual([entry["phase"] for entry in self.made], ["A", "B", "B", "B", "C"])

    def test_the_pilot_is_its_games_with_their_groups_the_dearest_first(self):
        # The cost of PILOT[k] is 10 + k: 3420 is the dearest; 3300 has the group 3500, 3501, 3510.
        self.assertEqual(self.lists["pilot7"]["games"], [3420, 3408, 3336, 3335, 3300, 3500, 3501, 3510, 3175, 2340])
        self.assertEqual(self.lists["pilot7"]["groups"], 7)
        self.assertAlmostEqual(self.lists["pilot7"]["hours"], sum(10.0 + k for k in range(7)))

    def test_the_dear_list_is_the_dearest_heads_the_pilot_and_the_deferred_leave(self):
        # The twenty dearest heads are 3600..3619; 3601 repeats a search and goes to the deferred list.
        self.assertEqual(self.lists["dear_rest"]["games"], [3600 + k for k in range(20) if k != 1])
        self.assertEqual(self.lists["dear_rest"]["groups"], 19)

    def test_the_deferred_list_holds_the_unsettled_repeats_and_their_groups_by_increasing_n(self):
        self.assertEqual(self.lists["deferred"]["games"], [3601, 3720, 3721])
        self.assertEqual(self.lists["deferred"]["groups"], 2)
        self.assertAlmostEqual(self.lists["deferred"]["hours"], 99.0 + 2.0)

    def test_the_rest_lists_hold_the_other_groups_balanced_each_in_increasing_n(self):
        rest = self.lists["rest_1"]["games"] + self.lists["rest_2"]["games"]
        self.assertEqual(sorted(rest), [3620, 3621] + list(range(3700, 3708)) + [3710])
        for name in ("rest_1", "rest_2"):
            self.assertEqual(self.lists[name]["games"], sorted(self.lists[name]["games"]))
        holder = "rest_1" if 3700 in self.lists["rest_1"]["games"] else "rest_2"
        self.assertIn(3710, self.lists[holder]["games"], "3700's group is whole in one list")
        # The dearest group to the lightest list: 3620 and 3621 apart, the cheap ones evening them out.
        self.assertLess(abs(self.lists["rest_1"]["hours"] - self.lists["rest_2"]["hours"]), 2.0)
        self.assertEqual(self.lists["rest_1"]["games"][0], 3620)

    def test_every_game_that_needs_a_certificate_is_in_one_list(self):
        placed = sorted(n for entry in self.made for n in entry["games"])
        self.assertEqual(placed, sorted(g["n"] for g in self.games if g["needs"] == 1))
        self.assertEqual(len(placed), 43)

    def test_the_synthetic_lists_pass_their_own_check(self):
        found = {entry["name"]: entry["games"] for entry in self.made}
        self.assertEqual(certify_campaign.check_lists(self.games, found), [])

    def test_a_pilot_game_that_the_rule_settles_or_that_is_deferred_or_missing_is_refused(self):
        repeat = [dict(g, proof="same_search") if g["n"] == PILOT[0] else g for g in self.games]
        with self.assertRaises(certify_campaign.CampaignError) as raised:
            certify_campaign.campaign_lists(repeat, 2)
        self.assertIn(str(PILOT[0]), str(raised.exception))
        shared = [dict(g, share=PILOT[1], proof="same_search") if g["n"] == PILOT[3] else g for g in self.games]
        with self.assertRaises(certify_campaign.CampaignError) as raised:
            certify_campaign.campaign_lists(shared, 2)
        self.assertIn(str(PILOT[3]), str(raised.exception))
        missing = [g for g in self.games if g["n"] != PILOT[0]]
        with self.assertRaises(certify_campaign.CampaignError):
            certify_campaign.campaign_lists(missing, 2)

    def test_the_list_text_has_four_header_lines_and_a_board_a_line(self):
        lines = certify_campaign.list_text(self.lists["deferred"], "cmd").splitlines()
        self.assertEqual([line.startswith("# ") for line in lines], [True] * 4 + [False] * 3)
        self.assertEqual(lines[1], "# games: 3 in 2 groups")
        self.assertEqual(lines[2], "# expected thread-hours: 101.00")
        self.assertEqual(lines[3], "# command: cmd")
        self.assertEqual(lines[4:], ["3601", "3720", "3721"])


class Check(unittest.TestCase):
    def setUp(self):
        self.games = synthetic()
        self.good = {entry["name"]: list(entry["games"]) for entry in certify_campaign.campaign_lists(self.games, 2)}

    def problems(self, mutate):
        found = {name: list(boards) for name, boards in self.good.items()}
        mutate(found)
        return certify_campaign.check_lists(self.games, found)

    @staticmethod
    def holder(lists, n):
        return next(name for name, boards in lists.items() if n in boards)

    def test_a_board_in_two_lists(self):
        found = self.problems(lambda lists: lists["rest_1"].append(3720))
        self.assertIn("board 3720 is in rest_1 and in deferred", found)

    def test_a_game_in_no_list(self):
        found = self.problems(lambda lists: lists[self.holder(lists, 3621)].remove(3621))
        self.assertEqual(found, ["board 3621 needs a certificate and is in no list"])

    def test_a_board_that_needs_no_certificate(self):
        found = self.problems(lambda lists: lists["rest_2"].append(3730))
        self.assertEqual(found, ["board 3730 of rest_2 does not need a certificate"])

    def test_a_game_that_shares_in_another_list_than_the_game_it_shares_with(self):
        def move(lists):
            lists["pilot7"].remove(3500)
            lists["rest_1"].append(3500)

        found = self.problems(move)
        self.assertIn("board 3500 shares with 3300, which is in pilot7, not in rest_1", found)

    def test_a_game_that_shares_before_the_game_it_shares_with(self):
        def swap(lists):
            boards = lists["pilot7"]
            a, b = boards.index(3300), boards.index(3500)
            boards[a], boards[b] = boards[b], boards[a]

        self.assertIn("board 3500 comes before 3300, whose certificate it rests on, in pilot7", self.problems(swap))

    def test_a_chain_of_shares_is_checked_link_by_link(self):
        # 3510 shares with 3500, which shares with 3300: the second link alone out of order.
        def swap(lists):
            boards = lists["pilot7"]
            a, b = boards.index(3500), boards.index(3510)
            boards[a], boards[b] = boards[b], boards[a]

        found = self.problems(swap)
        self.assertEqual(found, ["board 3510 comes before 3500, whose certificate it rests on, in pilot7"])

    def test_the_deferred_list_must_be_the_unsettled_repeats_with_their_groups(self):
        wrong = "deferred is not the unsettled repeats with their groups, by increasing n"
        self.assertIn(wrong, self.problems(lambda lists: lists["deferred"].reverse()))

        def move(lists):
            lists["deferred"].remove(3601)
            lists["dear_rest"].append(3601)

        self.assertIn(wrong, self.problems(move))

    def test_a_directory_of_lists_is_read_as_certify_reads_them(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            (directory / "one.txt").write_text("# header\n\n  12  \n# a comment\n13\n")
            (directory / "ignored.md").write_text("not a list")
            self.assertEqual(certify_campaign.directory_lists(directory), {"one": [12, 13]})
            for text, message in [("12\n12\n", "listed again (first on line 1)"), ("12\nx\n", "expected a board size"),
                                  ("12 13\n", "expected a board size"), ("-1\n", "expected a board size")]:
                (directory / "one.txt").write_text(text)
                with self.assertRaises(certify_campaign.CampaignError) as raised:
                    certify_campaign.directory_lists(directory)
                self.assertIn(message, str(raised.exception))
                self.assertIn("one.txt line", str(raised.exception))
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(certify_campaign.CampaignError):
                certify_campaign.directory_lists(tmp)


class CommandLine(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)
        self.plan = self.dir / "plan.jsonl"
        self.plan.write_text("".join(json.dumps(dict(g, extra=1)) + "\n" for g in synthetic()))

    def run_main(self, *argv):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = certify_campaign.main(list(argv))
        return code, out.getvalue(), err.getvalue()

    def test_lists_writes_the_files_and_check_passes_them(self):
        lists = self.dir / "lists"
        code, out, err = self.run_main("lists", "--plan", str(self.plan), "--out-dir", str(lists), "--rest", "3")
        self.assertEqual((code, err), (0, ""))
        written = ["dear_rest.txt", "deferred.txt", "pilot7.txt", "rest_1.txt", "rest_2.txt", "rest_3.txt"]
        self.assertEqual(sorted(p.name for p in lists.iterdir()), written)
        self.assertIn("pilot7", out)
        self.assertIn("(43 games need a certificate)", out)
        self.assertIn("certify_campaign.py lists --plan", (lists / "pilot7.txt").read_text())
        code, out, err = self.run_main("check", "--plan", str(self.plan), "--lists-dir", str(lists))
        self.assertEqual((code, err), (0, ""))
        self.assertTrue(out.startswith("6 lists, 43 games: "), out)

    def test_check_names_each_problem_and_exits_1(self):
        lists = self.dir / "lists"
        self.run_main("lists", "--plan", str(self.plan), "--out-dir", str(lists))
        (lists / "extra.txt").write_text("3720\n3730\n")
        code, _out, err = self.run_main("check", "--plan", str(self.plan), "--lists-dir", str(lists))
        self.assertEqual(code, 1)
        self.assertIn("board 3720 is in deferred and in extra", err)
        self.assertIn("board 3730 of extra does not need a certificate", err)

    def test_bad_inputs_are_errors_naming_the_file_and_line(self):
        self.plan.write_text("{}\n")
        code, _out, err = self.run_main("lists", "--plan", str(self.plan), "--out-dir", str(self.dir / "x"))
        self.assertEqual(code, 1)
        self.assertIn("plan.jsonl line 1", err)
        code, _out, err = self.run_main("lists", "--plan", str(self.dir / "none.jsonl"), "--out-dir", str(self.dir))
        self.assertEqual(code, 1)
        self.assertIn("cannot read", err)
        self.plan.write_text("".join(json.dumps(g) + "\n" for g in synthetic()))
        code, _out, err = self.run_main("lists", "--plan", str(self.plan), "--out-dir", str(self.dir), "--rest", "0")
        self.assertEqual(code, 1)
        self.assertIn("--rest must be at least 1", err)


class Committed(unittest.TestCase):
    """The lists as committed, against the work list as committed."""

    @classmethod
    def setUpClass(cls):
        cls.games = certify_campaign.read_plan(PLAN)
        cls.cost = {g["n"]: g["expected_thread_hours"] for g in cls.games}
        cls.on_disk = certify_campaign.directory_lists(STAGE4)
        cls.made = certify_campaign.campaign_lists(cls.games)

    def test_the_seven_files(self):
        self.assertEqual(sorted(self.on_disk), NAMES)

    def test_no_game_is_in_two_files_and_every_game_that_needs_a_certificate_is_in_one(self):
        self.assertEqual(certify_campaign.check_lists(self.games, self.on_disk), [])
        placed = [n for boards in self.on_disk.values() for n in boards]
        self.assertEqual(len(placed), len(set(placed)))
        self.assertEqual(sorted(placed), [g["n"] for g in self.games if g["needs"] == 1])
        self.assertEqual(len(placed), 708)

    def test_a_game_that_shares_follows_the_game_it_shares_with_in_the_same_file(self):
        where = {n: (name, i) for name, boards in self.on_disk.items() for i, n in enumerate(boards)}
        shared = [g for g in self.games if g["share"] is not None]
        self.assertEqual(len(shared), 145)
        for g in shared:
            before, after = where[g["share"]], where[g["n"]]
            self.assertEqual(before[0], after[0], f"{g['n']} shares with {g['share']}")
            self.assertLess(before[1], after[1], f"{g['n']} shares with {g['share']}")

    def test_the_files_are_what_the_tool_makes_of_the_work_list(self):
        for entry in self.made:
            self.assertEqual(self.on_disk[entry["name"]], entry["games"], entry["name"])
            header = (STAGE4 / f"{entry['name']}.txt").read_text().splitlines()[:3]
            self.assertEqual(header, certify_campaign.list_text(entry, "").splitlines()[:3])

    def test_the_pilot_is_the_seven_games_and_their_groups(self):
        pilot = self.on_disk["pilot7"]
        self.assertEqual(sorted(pilot), [2340, 3175, 3300, 3335, 3336, 3337, 3408, 3409, 3420, 3421])
        self.assertEqual(pilot[:2], [3300, 2340], "the dearest first")
        self.assertAlmostEqual(sum(self.cost[n] for n in pilot), 288.0, delta=0.05)

    def test_phase_b_starts_with_3440_3424_3425_and_2346_and_costs_2652_thread_hours(self):
        self.assertEqual(self.on_disk["dear_rest"][:4], [3440, 3424, 3425, 2346])
        phase_b = [n for name, boards in self.on_disk.items() if name not in ("pilot7", "deferred") for n in boards]
        self.assertAlmostEqual(sum(self.cost[n] for n in phase_b), 2652.0, delta=1.0)

    def test_the_deferred_list_is_the_115_repeats_the_rule_cannot_settle_with_their_groups(self):
        deferred = self.on_disk["deferred"]
        unsettled = sorted(certify_campaign.unsettled_repeats(self.games))
        self.assertEqual(len(unsettled), 115)
        self.assertEqual(deferred, sorted(deferred))
        self.assertTrue(set(unsettled) <= set(deferred))
        self.assertEqual(sorted(set(deferred) - set(unsettled)), [2519, 2735], "the games that share with two of them")
        self.assertIn(3427, deferred)
        self.assertNotIn(3427, self.on_disk["dear_rest"])
        self.assertAlmostEqual(sum(self.cost[n] for n in deferred), 361.2, delta=0.05)

    def test_the_rest_lists_are_balanced_and_in_increasing_n(self):
        totals = [sum(self.cost[n] for n in self.on_disk[f"rest_{i}"]) for i in range(1, 5)]
        self.assertLess(max(totals) - min(totals), 0.5)
        for i in range(1, 5):
            self.assertEqual(self.on_disk[f"rest_{i}"], sorted(self.on_disk[f"rest_{i}"]))

    def test_the_old_lists_of_the_plan_are_the_same_games_cut_another_way(self):
        old = certify_campaign.directory_lists(STAGE4.parent)
        self.assertEqual(sorted(n for boards in old.values() for n in boards),
                         sorted(n for boards in self.on_disk.values() for n in boards))


if __name__ == "__main__":
    unittest.main()
