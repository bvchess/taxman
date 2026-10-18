#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The pilot of the certificates campaign: each searched game's overhead against the plan, and the stop rule.

Standard library only; every path comes from the arguments or, by default, from the repository this file is in.

The plan (solver/tools/certify/plan.md, plan.jsonl) priced a certificate at twice the plain search of its problem, and a
plain search by its nodes at the cost per node c of the stretch of the chain it came from.  The pilot (phase A of the
campaign, solver/tools/certify/lists/stage4/pilot7.txt) runs seven of the dearest games first, to see what the overhead
is in the regime the money goes to.  For each game that `taxman certify` searched (a line with a verdict and no
`same_problem`), with N the nodes of its search in the chain (plan.jsonl `cost_nodes`: `chain_nodes` of an exhausted
game, the source's nodes of a repeat) and c its stretch's cost per node (plan.md's table of eras):

  R        = the line's thread-seconds / (N x c), the thread-seconds being `seconds_all_runs` x `threads`: the plan's
             factor, which it priced at 2
  R_nodes  = the line's nodes / N: the tree, against the chain's
  R_node   = the line's thread-microseconds per node / c: the node, against the chain's;  R = R_nodes x R_node

The stop rule: GO if the median R of the pilot's games is at most 3 and none is above 5 (then the campaign costs at
most 1.5 times the plan); STOP otherwise (report, and build the speedups before spending more); COUNTEREXAMPLE if any
line is `beaten` (a game that beats a recorded score: stop at once).  While games of the list have no verdict yet the
answer is UNFINISHED, unless a game is already above 5 (STOP) or a line is beaten.

Games that rest on another game's certificate by the rule of the same problem (`same_problem`) have no search: they are
counted, not rated.  A game whose N is not a finished search's (the plan priced it by the cost model) is shown, not
rated.  The report also gives, for each searched game, its |O| (the open members), its root gap (from the barred
bounds, `solver/noframe/data/upper_*.jsonl`) and its GF(2) block words (the problems file), the search time in
thread-hours against the plan's, and what that is on one machine at the plan's prices.

usage: certify_pilot.py CERTS... [--list pilot7.txt] [--plan plan.jsonl] [--plan-md plan.md] [--upper FILE...]
           [--problems FILE...] [--bands]
Exit status: 0 GO, 3 STOP, 2 COUNTEREXAMPLE, 4 UNFINISHED, 1 an input is wrong (the message names the file and line).
"""
import argparse
import re
import statistics
import sys
from pathlib import Path
from typing import NamedTuple, Optional

sys.path.insert(0, str(Path(__file__).resolve().parent))  # python -I leaves the script's own directory off the path
import certify_campaign  # noqa: E402
from certify_plan import INT, NUMBER, PlanError, need, read_lines  # noqa: E402

TOOLS = Path(__file__).resolve().parent
CERTIFY = TOOLS / "certify"  # the plan, the problems and the lists of the campaign
DATA = TOOLS.parent / "noframe" / "data"  # the prover's files
GO_MEDIAN = 3.0  # go on if the median R is at most this
GO_MAXIMUM = 5.0  # and no R is above this
PLANNED_R = 2.0  # the plan's overhead, which the report compares with
EXIT = {"GO": 0, "STOP": 3, "COUNTEREXAMPLE": 2, "UNFINISHED": 4}
MEASURED = ("chain", "source")  # the bases of a plan line whose nodes are those of a search that finished
GAP_BAND = 100
OPEN_BAND = 50
ERA_SECTION = "## The eras: what a node costs"
MACHINE = re.compile(r"^- machine: (\d+) threads", re.MULTILINE)
PRICES = re.compile(r"^- prices: \$([\d.]+) an hour on demand, \$([\d.]+) an hour at spot", re.MULTILINE)


class Era(NamedTuple):
    lo: int
    hi: int
    c: float  # microseconds of a thread a node


class PlanText(NamedTuple):
    """What the report takes from plan.md."""

    eras: list
    machine_threads: int
    on_demand: float
    spot: float


class Searched(NamedTuple):
    """A game that `taxman certify` searched, with what the plan says of it."""

    n: int
    open: int
    width: int
    nodes: int
    thread_seconds: float
    chain_nodes: Optional[int]
    c: float
    basis: Optional[str]
    plan_hours: float
    gap: Optional[float]
    words: Optional[int]

    @property
    def rated(self):
        """Whether the plan's nodes are those of a search that finished, so that the game has an R."""
        return self.basis in MEASURED and bool(self.chain_nodes)

    @property
    def r_nodes(self):
        return self.nodes / self.chain_nodes if self.rated else None

    @property
    def r_node(self):
        return self.thread_seconds * 1e6 / self.nodes / self.c if self.rated and self.nodes else None

    @property
    def r(self):
        return self.thread_seconds * 1e6 / (self.chain_nodes * self.c) if self.rated else None

    @property
    def hours(self):
        return self.thread_seconds / 3600


class Verdict(NamedTuple):
    word: str
    why: str


def read_plan_md(path):
    """The eras (their cost per node in microseconds), the machine's threads and the prices of plan.md."""
    try:
        text = Path(path).read_text()
    except (OSError, UnicodeDecodeError) as e:
        raise PlanError(f"{path}: cannot read: {getattr(e, 'strerror', None) or e}") from e
    if ERA_SECTION not in text:
        raise PlanError(f"{path}: no section {ERA_SECTION[3:]!r}")
    eras = []
    for line in text.split(ERA_SECTION, 1)[1].split("\n## ", 1)[0].splitlines():
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if line.startswith("|") and re.fullmatch(r"\d+-\d+", cells[0]):
            lo, hi = cells[0].split("-")
            eras.append(Era(int(lo), int(hi), float(cells[-1].replace(",", ""))))
    machine, prices = MACHINE.search(text), PRICES.search(text)
    if not eras or machine is None or prices is None:
        raise PlanError(f"{path}: cannot read the eras, the machine and the prices from it")
    return PlanText(eras, int(machine[1]), float(prices[1]), float(prices[2]))


def read_by_board(paths, key, wanted):
    """{n: the line's `key`} for the boards `wanted` over JSON-lines files, a later line replacing an earlier one."""
    found = {}
    for path in paths:
        for number, row in read_lines(path):
            n = need(row, "n", INT, f"{path} line {number}", low=1)
            if n in wanted:
                found[n] = row.get(key)
    return found


def read_certificates(paths):
    """({board: its last line with a final verdict}, {board: its `incomplete` lines}) over the files in order.  A line
    must be one of `taxman certify`'s: the fields the report uses are checked."""
    final, stopped = {}, {}
    for path in paths:
        for number, row in read_lines(path):
            n = need(row, "n", INT, f"{path} line {number}", low=1)
            where = f"{path} line {number} (board {n})"
            verdict = need(row, "verdict", str, where)
            if verdict not in ("certified", "beaten", "incomplete"):
                raise PlanError(f"{where}: unknown verdict {verdict!r}")
            for key in ("nodes", "threads", "width"):
                need(row, key, INT, where, low=0)
            for key in ("seconds", "seconds_all_runs"):
                need(row, key, NUMBER, where, low=0)
            need(row, "open", list, where)
            if verdict == "incomplete":
                stopped.setdefault(n, []).append(row)
            else:
                final[n] = row
    return final, stopped


def rate(n, row, plan_lines, plan, gaps, words):
    """The Searched of the certificate line `row` of board n, against the plan's line for the board."""
    era = next((e for e in plan.eras if e.lo <= n <= e.hi), None)
    if era is None:
        raise PlanError(f"board {n}: no era of plan.md holds it")
    line = plan_lines.get(n)
    if line is None:
        raise PlanError(f"board {n}: plan.jsonl has no line for it")
    if line.get("needs") != 1:
        raise PlanError(f"board {n}: plan.jsonl says it needs no certificate")
    return Searched(n, len(row["open"]), row["width"], row["nodes"], row["seconds_all_runs"] * row["threads"],
                    line.get("cost_nodes"), era.c, line.get("basis"), line.get("expected_thread_hours") or 0.0,
                    gaps.get(n), words.get(n))


def judge(rated, beaten, missing):
    """The stop rule: over the games with an R, the boards whose line is `beaten`, and the boards of the list that have
    no verdict yet."""
    if beaten:
        boards = ", ".join(map(str, sorted(beaten)))
        return Verdict("COUNTEREXAMPLE", f"a game beats its recorded score: board {boards}")
    worst = max(rated, key=lambda game: game.r, default=None)
    if worst is not None and worst.r > GO_MAXIMUM:
        return Verdict("STOP", f"board {worst.n} has R {worst.r:.2f}, above {GO_MAXIMUM:g}")
    if missing:
        shown = ", ".join(map(str, missing[:8])) + (", ..." if len(missing) > 8 else "")
        return Verdict("UNFINISHED", f"{len(missing)} games of the list have no verdict yet: {shown}")
    if worst is None:
        return Verdict("UNFINISHED", "no searched game to rate")
    median = statistics.median(game.r for game in rated)
    if median > GO_MEDIAN:
        return Verdict("STOP", f"the median R is {median:.2f}, above {GO_MEDIAN:g}")
    return Verdict("GO", f"the median R is {median:.2f}, at most {GO_MEDIAN:g}, and the largest {worst.r:.2f}, at most "
                         f"{GO_MAXIMUM:g}")


def fixed(x):
    return "-" if x is None else f"{x:.2f}"


def table(headers, rows, align):
    """Left- or right-aligned columns (one letter each, l or r) of strings."""
    widths = [max(len(str(cell)) for cell in column) for column in zip(headers, *rows)]

    def line(row):
        cells = [str(cell).rjust(w) if a == "r" else str(cell).ljust(w) for cell, w, a in zip(row, widths, align)]
        return "  ".join(cells).rstrip()

    return [line(headers)] + [line(row) for row in rows]


def game_table(games):
    headers = ["n", "|O|", "gap", "words", "width", "nodes", "chain nodes", "R_nodes", "R_node", "R", "thread-hours",
               "plan hours", "basis"]
    rows = [[g.n, g.open, "-" if g.gap is None else f"{g.gap:g}", "-" if g.words is None else g.words, g.width,
             f"{g.nodes:,}", "-" if g.chain_nodes is None else f"{g.chain_nodes:,}", fixed(g.r_nodes), fixed(g.r_node),
             fixed(g.r), fixed(g.hours), fixed(g.plan_hours), g.basis if g.rated else f"{g.basis} (not rated)"]
            for g in sorted(games, key=lambda g: g.n)]
    return table(headers, rows, "rrrrrrrrrrrrl")


def band_table(games, key, width, name):
    """The median and the largest R of the rated games by band of `key`."""
    bands = {}
    for g in games:
        if key(g) is not None:
            bands.setdefault(int(key(g)) // width * width, []).append(g.r)
    rows = [[f"{start}-{start + width - 1}", len(rs), fixed(statistics.median(rs)), fixed(max(rs))]
            for start, rs in sorted(bands.items())]
    return table([name, "games", "median R", "largest R"], rows, "lrrr")


def totals(plan, searched):
    """The lines about the search time: against the plan's and in dollars on one machine."""
    actual, planned = sum(g.hours for g in searched), sum(g.plan_hours for g in searched)
    machine = actual / plan.machine_threads
    line = f"search time: {actual:,.1f} thread-hours"
    if planned:
        line += f" against the plan's {planned:,.1f} for the same games ({actual / planned:.2f} times)"
    dollars = f"${machine * plan.spot:,.2f} at spot or ${machine * plan.on_demand:,.2f} on demand"
    return [line, f"on {plan.machine_threads} threads that is {machine:,.2f} hours, {dollars} (the search alone: the "
                  "bill is the instance's hours)"]


def report(plan, searched, rule, beaten, stopped, missing, outside, bands):
    """The lines of the report, and its Verdict."""
    rated = [g for g in searched if g.rated]
    out = game_table(searched) + [""] if searched else []
    out.append(f"{len(searched)} games searched, {len(rule)} settled by the rule of the same problem, "
               f"{len(beaten)} beaten" + (f"; {len(stopped)} stopped once or more and resumed" if stopped else ""))
    if rated:
        rs = [g.r for g in rated]
        top = max(rated, key=lambda g: g.r)
        out.append(f"R over {len(rated)} rated games: median {statistics.median(rs):.2f}, largest {top.r:.2f} "
                   f"(board {top.n}), smallest {min(rs):.2f}; the plan priced {PLANNED_R:g}")
        nodes, node = [g.r_nodes for g in rated], [g.r_node for g in rated if g.r_node is not None]
        out.append(f"R_nodes: median {statistics.median(nodes):.2f}" +
                   (f"; R_node: median {statistics.median(node):.2f}" if node else ""))
    if searched:
        out += totals(plan, searched)
    if bands and rated:
        out += ["", *band_table(rated, lambda g: g.gap, GAP_BAND, "gap"), "", *band_table(rated, lambda g: g.open,
                                                                                         OPEN_BAND, "|O|")]
    if outside:
        out.append(f"note: {len(outside)} games with a verdict are not in the list: "
                   + ", ".join(map(str, sorted(outside)[:8])))
    verdict = judge(rated, beaten, missing)
    return out + ["", f"stop rule: {verdict.word}: {verdict.why}"], verdict


def parse_args(argv):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("certificates", nargs="+", metavar="CERTS", help="files of certificates (taxman certify --out)")
    ap.add_argument("--list", type=Path, default=None, metavar="FILE", help="the pilot's list: the games to wait for")
    ap.add_argument("--plan", default=str(CERTIFY / "plan.jsonl"), metavar="FILE")
    ap.add_argument("--plan-md", default=str(CERTIFY / "plan.md"), metavar="FILE")
    ap.add_argument("--upper", nargs="+", default=sorted(map(str, DATA.glob("upper_*.jsonl"))),
                    metavar="FILE", help="the barred bounds, for the root gaps")
    ap.add_argument("--problems", nargs="+", default=[str(CERTIFY / "problems_600_3440.jsonl")],
                    metavar="FILE", help="the extended problems, for the GF(2) block words")
    ap.add_argument("--bands", action="store_true", help="also the median R by band of gap and of |O|")
    return ap.parse_args(argv)


def run(args):
    plan = read_plan_md(args.plan_md)
    found, stopped = read_certificates(args.certificates)
    listed = None if args.list is None else certify_campaign.read_list(args.list)
    plan_lines = {row["n"]: row for _number, row in read_lines(args.plan) if "n" in row}
    boards = set(found) | set(listed or [])
    gaps, words = read_by_board(args.upper, "gap", boards), read_by_board(args.problems, "words", boards)
    searched = [rate(n, row, plan_lines, plan, gaps, words) for n, row in sorted(found.items())
                if row["verdict"] == "certified" and "same_problem" not in row]
    rule = [n for n, row in found.items() if row["verdict"] == "certified" and "same_problem" in row]
    beaten = [n for n, row in found.items() if row["verdict"] == "beaten"]
    missing = [] if listed is None else [n for n in listed if n not in found]
    outside = [] if listed is None else [n for n in found if n not in listed]
    return report(plan, searched, rule, beaten, stopped, missing, outside, args.bands)


def main(argv=None):
    args = parse_args(sys.argv[1:] if argv is None else list(argv))
    try:
        lines, verdict = run(args)
    except (PlanError, certify_campaign.CampaignError) as e:
        print(f"certify_pilot.py: error: {e}", file=sys.stderr)
        return 1
    print("\n".join(lines))
    return EXIT[verdict.word]


if __name__ == "__main__":
    sys.exit(main())
