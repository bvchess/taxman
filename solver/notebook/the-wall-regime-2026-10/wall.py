#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The wall regime's tables: docs/experiments/the-wall-regime-2026-10.md.

What a game of the chain costs is its search, which grows with the size of the tree, plus everything around the search,
which grows with the board size n alone.  `examples/wall.rs` measures the two apart; its output is in this directory
(JSON lines of `wall setup`, `nodes`, `grow`, `start` and `frames`, and the progress lines of `grow`; the fields are
in the example's module doc).  This script joins that data with the committed chain `../../results/strict.jsonl` and the
c8g chain `../cloud/c8g.chain-2176.jsonl`, and prints one markdown table per question, each with a heading and a
sentence.  Pure Python; the inputs are found relative to this file, so it runs from anywhere:

    ./wall.py > wall_output.md      every table
    ./wall.py setup nodes           only the named ones
    ./wall.py --chain-end 3440      the committed chain's games 1..3440 instead of 1..CHAIN_END

The tables of the committed chain (phases, zero, fixed and check) take its games 1..CHAIN_END = 2175, the chain as it
was when `wall_output.md` was made; `strict.jsonl` has grown since.

The tables, with their inputs (files of this directory unless a path says otherwise):

* phases: the committed chain's seconds by phase, method and band of games (strict.jsonl).
* zero: the searched games of the chain that took no nodes, against those that did (strict.jsonl).
* setup: the fixed costs of a game piece by piece, and its sizes, at the set width the solver picks and at other
  widths (setup.jsonl, setup_widths.jsonl).
* frames: what the board builder's frames cost for boards up to 8191 (frames.jsonl).
* fixed: the model of the fixed cost of a game by method, checked against the chain and summed over 2176..4095
  (strict.jsonl, setup.jsonl).
* check: the model of the time of the results check, to 4095 (strict.jsonl, setup.jsonl).
* nodes: the cost of a node at the root-sized part of a tree (nodes_start_low, nodes_opt_low, nodes_start_high and
  ckpt_4000.jsonl).
* widths: the same trees at several set widths, and a least-squares line of cycles a node against W (widths_1300,
  widths_2170 and widths_3000.jsonl).
* start: the start's local search, singles against pairs, by band (start.jsonl).
* gaps: the root bound's gaps over the start incumbent and the cost of a node, with trends in n (gaps.jsonl with
  nodes_start_low and nodes_start_high.jsonl), and the dear games past 2175 against the c8g chain's optima, with no
  trends (gaps_c8g.jsonl, seeded starts).
* series: the node rate through a long run, by tenth of the run and by phase of the period of forgetting nogoods
  (grow_1430.progress, grow_1230_matching.progress).
* grow: the summary lines of the long runs and their largest checkpoints (grow_1430, grow_1230_matching and
  grow_2170_t64c1.jsonl and .progress).
* c8g: the c8g chain past 2175, by method and its dearest games (../cloud/c8g.chain-2176.jsonl).
"""
import bisect
import collections
import functools
import json
import math
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
RUST = HERE.parents[1]
CHAIN = RUST / "results" / "strict.jsonl"
C8G_CHAIN = RUST / "bench" / "cloud" / "c8g.chain-2176.jsonl"
CHAIN_END = 2175  # the last game of the committed chain that the tables use (wall_output.md: the chain ended here)

PHASE_BAND = 250  # games to a band in the tables of the committed chain
START_BAND = 175  # games to a band in the table of the start's local search
LATE = 1300  # the games past this one (beyond optimal.json) are the ones whose mix of methods the later games follow
CHECK_FROM = 2001  # the model of the fixed costs is checked against the chain's games from this one to its last
TARGET = 4095  # the last board of the widest set the solver supported at the study (8191 since boards past 4095)
WIDEST = 64  # words of that set, which holds the boards up to TARGET
PERIOD = 50_000  # steps a searcher takes between two forgettings of the nogoods
MB = 2**20
METHODS = ("prime", "repeat", "search")

# The pieces of a game's fixed cost that `wall setup` times (each reported as <piece>_s seconds).
SETUP_PIECES = ("tables", "frames", "plain", "board", "prepare", "problem", "search_problem", "match_graph", "improve",
                "zero_search", "gf2_graph", "root_bound", "root_split", "moves", "replay", "prime_path")

# The counters that are the same in the trees of one search at every set width.
TREE_COUNTERS = ("nodes", "promote_attempts", "promotions", "children", "gf2_solves", "gf2_augmentations",
                 "gf2_strong_solves", "gf2_strong_augmentations")

# One line of the engine's progress output: elapsed time as [Nm]S.Ss, the nodes taken, then the rate.
PROGRESS_LINE = re.compile(r"n=\d+ (?:(\d+)m)?([\d.]+)s nodes (\d+) \([\d.]+k?/s\)")


# ---------------------------------------------------------------------------------------------------------
# Helpers

@functools.cache
def load(path):
    """The JSON objects of a JSON-lines file.  A file is read once, so callers must not change the list."""
    return [json.loads(line) for line in Path(path).read_text().splitlines() if line.startswith("{")]


chain_end = CHAIN_END  # the last game of the committed chain in use: --chain-end sets it


def committed_chain():
    """The lines of the committed chain, games 1..chain_end."""
    return [r for r in load(CHAIN) if r["n"] <= chain_end]


def node_lines(*names):
    """The `wall nodes` lines of the named files of this directory, in order (a trivial board has no search)."""
    return [r for name in names for r in load(HERE / name) if "nodes" in r]


def progress_points(path):
    """(seconds, nodes) of each progress line of a `wall grow` run."""
    points = []
    for line in Path(path).read_text().splitlines():
        m = PROGRESS_LINE.search(line)
        if m:
            points.append((60 * int(m.group(1) or 0) + float(m.group(2)), int(m.group(3))))
    return points


def mean(values):
    values = list(values)
    return sum(values) / len(values)


def fit(points):
    """Least squares y = a + b x through (x, y) points: (a, b)."""
    n = len(points)
    mx = sum(x for x, _ in points) / n
    my = sum(y for _, y in points) / n
    b = sum((x - mx) * (y - my) for x, y in points) / sum((x - mx) ** 2 for x, _ in points)
    return my - b * mx, b


def band(n, width):
    """The first board of the band of `width` boards that holds n (the bands run 1..width, width+1..2*width, ...)."""
    return (n - 1) // width * width + 1


def band_label(first, width):
    return f"{first}-{first + width - 1}"


def interpolation(rows, key):
    """The function of n through the points (n, row[key]) of the rows that have the key: linear between points, and
    beyond them the end point scaled by (n / its n) squared below the first and cubed above the last (the early games
    of the chain are below the first measured board)."""
    points = sorted((r["n"], r[key]) for r in rows if key in r)
    ns = [x for x, _ in points]

    def at(n):
        k = bisect.bisect_left(ns, n)
        if k == 0:
            return points[0][1] * (n / ns[0]) ** 2
        if k == len(points):
            return points[-1][1] * (n / ns[-1]) ** 3
        (x0, y0), (x1, y1) = points[k - 1], points[k]
        return y0 + (y1 - y0) * (n - x0) / (x1 - x0)

    return at


def section(title, sentence, header, rows):
    """A markdown section: a heading, one sentence and a table (None shows as a dash)."""
    lines = [f"## {title}", "", sentence, "", "| " + " | ".join(header) + " |", "|" + "---|" * len(header)]
    lines += ["| " + " | ".join("-" if cell is None else str(cell) for cell in row) + " |" for row in rows]
    return "\n".join(lines)


# ---------------------------------------------------------------------------------------------------------
# The committed chain

def phases():
    games = collections.defaultdict(list)
    for r in committed_chain():
        seconds = r.get("seconds") or {}
        games[(band(r["n"], PHASE_BAND), r["method"])].append(
            [seconds.get(phase, 0) for phase in ("total", "search", "frames")])
    rows = []
    for (first, method), g in sorted(games.items()):
        total, search, frames = (sum(x[i] for x in g) for i in range(3))
        rows.append([band_label(first, PHASE_BAND), method, len(g), f"{total:.1f}", f"{search:.1f}", f"{frames:.1f}",
                     f"{total - search - frames:.1f}", f"{total / len(g):.3f}", f"{(total - search) / len(g):.3f}"])
    return section(
        "Phases of the committed chain",
        f"Seconds the committed chain spent in each phase, summed over the games of each band of {PHASE_BAND} and "
        "method (other is the total less the search and the frames), with the mean per game in all and apart from the "
        "search.",
        ["band", "method", "games", "total s", "search s", "frames s", "other s", "mean total s", "mean non-search s"],
        rows)


def zero_node_games():
    games = collections.defaultdict(list)
    for r in committed_chain():
        if r["method"] != "search":
            continue
        seconds = r.get("seconds") or {}
        search = seconds.get("search", 0)
        games[(band(r["n"], PHASE_BAND), r.get("nodes", 0) == 0)].append(
            (search, seconds.get("total", 0) - search, r["setup"]["candidates"]))
    rows = []
    for (first, no_nodes), g in sorted(games.items()):
        search, rest, candidates = (mean(x[i] for x in g) for i in range(3))
        rows.append([band_label(first, PHASE_BAND), "no nodes" if no_nodes else "nodes", len(g), f"{search:.3f}",
                     f"{rest:.3f}", f"{candidates:.0f}"])
    return section(
        "Searched games that took no nodes",
        "The search phase of a searched game that the ceiling from the game of n - 1 ended before its first node "
        "(the search's own setup is all it costs) against one that took nodes, by band, with the rest of the game and "
        "the candidates.",
        ["band", "the search", "games", "mean search s", "mean rest s", "mean candidates"], rows)


# ---------------------------------------------------------------------------------------------------------
# The fixed costs of a game

SETUP_FILES = (
    ("setup.jsonl", "", "at the set width the solver picks for n"),
    ("setup_widths.jsonl", " at other set widths", "with the game set up at the widths named (`wall setup --width`)"),
)


def setup():
    sections = []
    for name, title, how in SETUP_FILES:
        rows = [r for r in load(HERE / name) if not r.get("trivial")]
        sections.append(section(
            f"Fixed costs of a game{title}",
            f"Seconds that each piece of a game's cost besides its search takes (the fastest of the repeats), {how}.",
            ["n", "W"] + [piece.replace("_", " ") for piece in SETUP_PIECES],
            [[r["n"], r["width"]] + [f"{r[piece + '_s']:.3f}" for piece in SETUP_PIECES] for r in rows]))
        sections.append(section(
            f"Sizes of a game{title}",
            "The frames, candidates and members of each game and the words that hold them, the sizes of the root's "
            "GF(2) matroids, its bound with the gaps over the start incumbent and over the known optimum, whether the "
            f"ceiling ended the search, and the peak resident set, {how}.",
            ["n", "W", "frames", "candidates", "members", "words (members)", "words (candidates)", "max total",
             "elements", "ranks", "block words", "root bound", "start", "gap over start", "gap over opt",
             "ended by ceiling", "rss MB", "root augmentations"],
            [[r["n"], r["width"], r["frames"], r["candidates"], r["members"], r["words_members"],
              r["words_candidates"], r["max_total"], r["elements"], r["ranks"], r["block_words"], r["root_bound"],
              r["start"], r["gap_start"], r["gap_opt"], r["capped"], f"{r['max_rss'] / MB:.1f}",
              r["root_augmentations"]] for r in rows]))
    return "\n\n".join(sections)


def frames():
    return section(
        "Frames of the boards past the widest set",
        "The time the board builder's frames take for boards up to 8191, their sizes, and the words that a set and "
        "the GF(2) bound's vectors need there.",
        ["n", "set width", "frames", "candidates", "members", "words (members)", "words (candidates)", "max freedoms",
         "block words", "frames s"],
        [[r["n"], r["width"], r["frames"], r["candidates"], r["members"], r["words_members"], r["words_candidates"],
          r["max_freedoms"], r["block_words"], f"{r['frames_s']:.3f}"] for r in load(HERE / "frames.jsonl")])


def fixed_model():
    chain = committed_chain()
    last = chain[-1]["n"]
    setup_rows = load(HERE / "setup.jsonl")
    at = {key: interpolation(setup_rows, key) for key in (
        "board_s", "prepare_s", "problem_s", "search_problem_s", "moves_s", "replay_s", "zero_search_s",
        "prime_path_s")}

    def cost(n, method, seeded_share=1.0):
        """Model seconds of the fixed cost of game n by method: its board, its search problem or its search's own
        setup (a share of the unseeded measurement), the promotions made and two replays of the game's moves."""
        replays = 2 * at["replay_s"](n)
        board = at["board_s"](n) + at["prepare_s"](n) + at["problem_s"](n)
        if method == "prime":
            return at["prime_path_s"](n) + replays
        if method == "repeat":
            return board + at["search_problem_s"](n) + at["moves_s"](n) + replays
        return board + seeded_share * at["zero_search_s"](n) + at["moves_s"](n) + replays

    mix = collections.Counter(r["method"] for r in chain if r["n"] > LATE)
    total = sum(mix.values())
    recent = [r for r in chain if CHECK_FROM <= r["n"] <= last]
    calibration = []
    for method in METHODS:
        games = [r for r in recent if r["method"] == method]
        # The search seconds of a game that took nodes are the search itself; everything else is fixed.
        measured = mean(r["seconds"]["total"] - (r["seconds"]["search"] if r["nodes"] else 0) for r in games)
        calibration.append([method, f"{mix[method] / total:.3f}", f"{measured:.3f}",
                            f"{mean(cost(r['n'], method) for r in games):.3f}"])
    by_n = []
    for n in (2200, 2500, 3000, 3500, 4000, TARGET):
        mixed = sum(cost(n, m) * c / total for m, c in mix.items())
        by_n.append([n] + [f"{cost(n, m):.2f}" for m in METHODS] + [f"{mixed:.2f}"])
    sums = []
    for share in (1.0, 0.5):
        seconds = sum(cost(n, m, share) * c / total for n in range(last + 1, TARGET + 1) for m, c in mix.items())
        sums.append([share, f"{seconds:.0f}", f"{seconds / 3600:.2f}"])
    return "\n\n".join([
        section(
            "The model of a game's fixed cost against the committed chain",
            "A game's fixed cost is everything but its search, which is serial (one thread works while the others "
            f"wait); the model, built from the pieces of `wall setup`, against the committed chain's own times at "
            f"{CHECK_FROM}-{last}, with the share of each method in the games past {LATE}.",
            ["method", f"share of the games past {LATE}", f"measured s a game, {CHECK_FROM}-{last}",
             "model s a game"], calibration),
        section(
            "The model's fixed cost by method and board",
            "Model seconds of the fixed cost of one game of each method at a board size, and the mean over the "
            "chain's mix of methods.",
            ["n"] + list(METHODS) + ["mean"], by_n),
        section(
            f"The fixed costs summed over {last + 1}..{TARGET}",
            f"The model summed over the games {last + 1}..{TARGET} with the chain's mix of methods past {LATE}, for "
            "the setup of a searched game at the full unseeded measurement and at half of it (a seeded search's).",
            ["share of the unseeded setup", "seconds", "hours serial"], sums),
    ])


def check_model():
    chain = committed_chain()
    last = chain[-1]["n"]
    setup_rows = load(HERE / "setup.jsonl")
    search_problem = interpolation(setup_rows, "search_problem_s")
    replay = interpolation(setup_rows, "replay_s")
    boards = set()
    for r in chain:
        if r["method"] == "repeat":
            boards.add(r["n"])
            boards.add(r["same_search"]["n"])
    late = [r for r in chain if r["n"] > LATE]
    late_share = len({n for n in boards if n > LATE}) / len(late)
    setups = sum(search_problem(n) for n in boards)
    replays = sum(replay(r["n"]) for r in chain)
    rows = [[f"1..{last} (the committed chain)", len(boards), f"{100 * len(boards) / len(chain):.0f}%",
             f"{setups:.1f}", f"{replays:.1f}", f"{setups + replays:.1f}", f"{(setups + replays) / 60:.1f}"]]
    for top in (2500, 3000, 3500, TARGET):
        extra = sum(late_share * search_problem(n) + replay(n) for n in range(last + 1, top + 1))
        seconds = setups + replays + extra
        rows.append([f"1..{top} (extended)", None, None, None, None, f"{seconds:.0f}", f"{seconds / 60:.1f}"])
    return section(
        "The model of the time of the results check",
        "The check sets a search problem up again for every repeat line and the game it repeats (each board once) and "
        f"replays every line; the model is the measured cost of a setup and of a replay by n, extended past {last} "
        f"with the share of boards set up again past {LATE} ({100 * late_share:.0f}% of the lines), on one thread.",
        ["chain", "boards set up again", "share of the lines", "setups s", "replays s", "total s", "total min"], rows)


# ---------------------------------------------------------------------------------------------------------
# The cost of a node

def node_costs():
    rows = []
    for name in ("nodes_start_low", "nodes_opt_low", "nodes_start_high", "ckpt_4000"):
        for r in node_lines(f"{name}.jsonl"):
            nodes = max(r["nodes"], 1)
            augmentations = max(r["gf2_augmentations"], 1)
            rows.append([
                name, r["n"], r["mode"], r["width"], r["completed"], r["nodes"], f"{r['busy_s']:.2f}",
                r["us_per_node"], r["us_per_aug"],
                f"{r['cycles_per_node'] / 1e3:.0f}" if r["cycles_per_node"] else None,
                f"{r['gf2_augmentations'] / nodes:.1f}", f"{100 * r['gf2_strong_augmentations'] / augmentations:.0f}",
                f"{r['gf2_solves'] / nodes:.2f}", f"{r['children'] / nodes:.2f}", r["start"], r["best"], r["opt"],
                r["root_bound"], r["gap_start"], r["root_bound"] - r["best"], f"{r['max_rss'] / MB:.1f}"])
    return section(
        "The cost of a node",
        "One-thread searches of each game, stopped after about 5000 steps (`wall nodes`; mode start searches from the "
        "start incumbent, mode opt proves the known optimum): the cost of a node in time and cycles, the GF(2) work "
        "and the children of a node, and the root's gaps over the start incumbent and over the best set found.",
        ["file", "n", "mode", "W", "completed", "nodes", "busy s", "µs a node", "µs an augmentation",
         "kcycles a node", "augmentations a node", "strong % of augmentations", "solves a node", "children a node",
         "start", "best", "opt", "root bound", "gap over start", "gap over best", "rss MB"], rows)


def widths():
    per_width, lines = [], []
    for name in ("widths_1300", "widths_2170", "widths_3000"):
        rows = load(HERE / f"{name}.jsonl")
        trees = {tuple(r[k] for k in TREE_COUNTERS) for r in rows}
        by_width = collections.defaultdict(list)
        for r in rows:
            by_width[r["width"]].append(r)
        points = []
        for w in sorted(by_width):
            repeats = by_width[w]
            nodes = repeats[0]["nodes"]
            cycles = min(r["cycles"] for r in repeats) / nodes
            instructions = min(r["instructions"] for r in repeats) / nodes
            busy = min(r["busy_s"] for r in repeats) / nodes * 1e6
            points.append((w, cycles))
            per_width.append([name, rows[0]["n"], rows[0]["mode"], w, f"{cycles / 1e3:.1f}",
                              f"{instructions / 1e3:.1f}", f"{busy:.1f}",
                              f"{max(r['max_rss'] for r in repeats) / MB:.1f}"])
        a, b = fit(points)
        lines.append([name, len(trees) == 1, ", ".join(map(str, next(iter(trees)))) if len(trees) == 1 else None,
                      f"{a / 1e3:.1f}", f"{b / 1e3:.3f}", f"{100 * b * WIDEST / (a + b * WIDEST):.1f}",
                      f"{100 * (points[-1][1] / points[0][1] - 1):.1f}", f"W {points[-1][0]} over W {points[0][0]}"])
    return "\n\n".join([
        section(
            "The same searches at several set widths",
            "For each set width the fastest of the repeats of the same search, as kilocycles, kiloinstructions and "
            "microseconds of busy time a node.",
            ["file", "n", "mode", "W", "kcycles a node", "kinstructions a node", "busy µs a node", "rss MB"],
            per_width),
        section(
            "What the set width costs a node",
            "Whether the trees are identical at every width (the counters are nodes, promote attempts, promotions, "
            "children, GF(2) solves and augmentations, strong solves and strong augmentations) and a least-squares "
            f"line of cycles a node against W, with the share of a node at W={WIDEST} that the line puts on W.",
            ["file", "trees identical", "counters", "intercept kcycles", "kcycles per unit of W",
             f"W part of a node at W={WIDEST} %", "widest over narrowest %", "widths compared"], lines),
    ])


def start_search():
    games = collections.defaultdict(list)
    for r in load(HERE / "start.jsonl"):
        if "pairs" in r:
            games[band(r["n"], START_BAND)].append(r)
    rows = []
    for first, g in sorted(games.items()):
        gains = [r["pairs"] - r["singles"] for r in g if r["pairs"] > r["singles"]]
        rows.append([
            band_label(first, START_BAND), len(g), sum(r["at_ceiling"] for r in g),
            sum(r["base"] == r["opt"] for r in g), sum(r["singles"] > r["base"] for r in g),
            sum(r["singles"] == r["opt"] for r in g), len(gains),
            sum(r["pairs"] == r["opt"] for r in g), f"{mean(r['pairs'] - r['singles'] for r in g):.1f}",
            f"{mean(r['singles_s'] for r in g):.4f}", f"{mean(r['pairs_s'] for r in g):.3f}", gains])
    every = [r for g in games.values() for r in g]
    at_ceiling_s = sum(r["pairs_s"] for r in every if r["at_ceiling"])
    return section(
        "The start's local search",
        "For sampled games, by band, how often the local search from the base raises it by taking out one member "
        "(singles) or two (pairs) and how often it reaches the optimum, with the mean gain and time of each and the "
        f"gains where pairs beat singles; the pairs spent {at_ceiling_s:.2f} of {sum(r['pairs_s'] for r in every):.2f} "
        "seconds on games whose base was already at the ceiling.",
        ["band", "games", "base at ceiling", "base = opt", "singles > base", "singles = opt", "pairs > singles",
         "pairs = opt", "mean gain of pairs", "singles s", "pairs s", "gains of pairs over singles"], rows)


def gap_sections(rows, title, sentence, trends=True):
    """The gap table of the games of rows (a game in more than one file keeps its last line), and with trends, a
    table of the least-squares change per 100 in n of the gaps and of the cost of a node."""
    rows = sorted({r["n"]: r for r in rows}.values(), key=lambda r: r["n"])
    sections = [section(
        title, sentence,
        ["n", "W", "gap over start", "gap over opt", "start below opt", "gap over best", "µs a node",
         "augmentations a node", "µs an augmentation", "candidates", "max total"],
        [[r["n"], r["width"], r["gap_start"], r["gap_opt"], r["opt"] - r["start"] if r["opt"] else None,
          r["root_bound"] - r["best"], r["us_per_node"], f"{r['gf2_augmentations'] / r['nodes']:.1f}",
          r["us_per_aug"], r["candidates"], r["max_total"]] for r in rows])]
    if trends:
        known = [r for r in rows if r["opt"]]

        def slope(games, y):
            """Least-squares change of y per 100 in n over the games."""
            return 100 * fit([(r["n"], y(r)) for r in games])[1]

        trend_rows = [
            ["gap over start", len(rows), f"{slope(rows, lambda r: r['gap_start']):+.1f}"],
            ["gap over opt (games with a known optimum)", len(known), f"{slope(known, lambda r: r['gap_opt']):+.1f}"],
            ["log10 µs a node (decades)", len(rows), f"{slope(rows, lambda r: math.log10(r['us_per_node'])):+.3f}"],
            ["log10 µs an augmentation (decades)", len(rows),
             f"{slope(rows, lambda r: math.log10(r['us_per_aug'])):+.3f}"],
            ["log10 augmentations a node (decades)", len(rows),
             f"{slope(rows, lambda r: math.log10(r['gf2_augmentations'] / r['nodes'])):+.3f}"],
        ]
        below = [r["opt"] - r["start"] for r in known]
        sections.append(section(
            "Trends in n of the gaps and of the cost of a node",
            "Least-squares lines through the games above, as the change per 100 in n; the start incumbent is "
            f"{mean(below):.0f} below the optimum on average and {max(below)} at most over the {len(known)} games "
            "with a known optimum.",
            ["quantity", "games", "change per 100 in n"], trend_rows))
    return "\n\n".join(sections)


def gaps():
    return "\n\n".join([
        gap_sections(
            node_lines("gaps.jsonl", "nodes_start_low.jsonl", "nodes_start_high.jsonl"),
            "Gaps of the root's GF(2) bound and the cost of a node",
            "The root's GF(2) bound over the unseeded start incumbent (and over the optimum where it is known), how "
            "far the start is below the optimum, and the cost of a node, a game to a row, from gaps.jsonl, "
            "nodes_start_low.jsonl and nodes_start_high.jsonl (a game in more than one file keeps the last line)."),
        gap_sections(
            node_lines("gaps_c8g.jsonl"),
            "Gaps of the dear games past 2175, against the c8g chain's optima",
            "The root's GF(2) bound over the start incumbent (seeded from the game of n - 1) and over the optimum "
            "that the c8g chain found, how far the start is below it, and the cost of a node, a game to a row, from "
            "gaps_c8g.jsonl (measured on the laptop).",
            trends=False),
    ])


# ---------------------------------------------------------------------------------------------------------
# Long runs

def series():
    by_tenth, by_fifth, totals = [], [], []
    for name in ("grow_1430", "grow_1230_matching"):
        points = progress_points(HERE / f"{name}.progress")
        segments = [(n0, n1, t1 - t0) for (t0, n0), (t1, n1) in zip(points, points[1:]) if t1 > t0 and n1 > n0]
        total = points[-1][1]
        totals.append(f"{total} for {name}")
        for k in range(10):
            lo, hi = total * k / 10, total * (k + 1) / 10
            inside = [(n1 - n0, dt) for n0, n1, dt in segments if lo <= n0 < hi]
            if inside:
                by_tenth.append([name, int(lo), f"{1e6 * sum(dt for _, dt in inside) / sum(n for n, _ in inside):.1f}"])
        fifths = collections.defaultdict(lambda: [0, 0.0])
        for n0, n1, dt in segments:
            fifth = int(5 * ((n0 + n1) / 2 % PERIOD) / PERIOD)
            fifths[fifth][0] += n1 - n0
            fifths[fifth][1] += dt
        by_fifth += [[name, fifth, f"{1e6 * seconds / nodes:.1f}", nodes]
                     for fifth, (nodes, seconds) in sorted(fifths.items())]
    return "\n\n".join([
        section(
            "The node rate through a long run",
            "The microseconds a node of each long `wall grow` run (grow_1230_matching is the matching path) by tenth "
            f"of the nodes up to its last progress point ({' and '.join(totals)}), from its progress lines.",
            ["run", "nodes from", "µs a node"], by_tenth),
        section(
            "The node rate in the period of forgetting nogoods",
            f"The same rate by fifth of the period of forgetting nogoods ({PERIOD} steps a searcher; one thread), "
            "with the nodes in each fifth.",
            ["run", "fifth of the period", "µs a node", "nodes"], by_fifth),
    ])


def grow_runs():
    rows = []
    for name in ("grow_1430", "grow_1230_matching", "grow_2170_t64c1"):
        lines = load(HERE / f"{name}.jsonl")
        summary = next(r for r in lines if "completed" in r)
        monitor = [r["ckpt_bytes"] for r in lines if "t" in r and r["ckpt_bytes"] is not None]
        rows.append([
            name, summary["n"], summary["threads"], summary["cores"], summary["completed"], summary["nodes"],
            f"{summary['seconds']:.1f}", summary["pauses"], f"{1000 * summary['pause_max_s']:.1f}",
            f"{1000 * summary['park_max_s']:.1f}", summary["max_queue"], f"{summary['ckpt_bytes']:,}",
            f"{max(monitor):,}", f"{summary['max_rss'] / MB:.1f}", len(progress_points(HERE / f"{name}.progress"))])
    return section(
        "The long runs",
        "The summary line of each `wall grow` run, with the largest checkpoint that its monitor lines report and the "
        "number of points in its progress lines (grow_1230_matching is the matching path, grow_2170_t64c1 runs 64 "
        "threads taking turns on one CPU).",
        ["run", "n", "threads", "cores", "completed", "nodes", "seconds", "pauses", "longest pause ms",
         "longest park ms", "max queue", "last checkpoint bytes", "largest checkpoint bytes", "rss MB",
         "progress points"], rows)


def c8g_chain():
    lines = load(C8G_CHAIN)
    done = [r for r in lines if r.get("completed")]
    groups = collections.defaultdict(lambda: [0, 0.0, 0, 0.0])
    for r in done:
        g = groups[(r["method"], str(r.get("proof")))]
        g[0] += 1
        g[1] += r["seconds"]["search"]
        g[2] += r["nodes"]
        g[3] += r["seconds"]["total"]
    dearest = sorted(done, key=lambda r: -r["seconds"]["search"])[:10]
    return "\n\n".join([
        section(
            "The c8g chain past 2175, by method",
            f"The {len(done)} completed games of the {len(lines)} lines {lines[0]['n']}..{lines[-1]['n']} (64 threads) "
            "by method and proof, with their search seconds, nodes and total seconds.",
            ["method", "proof", "games", "search s", "nodes", "total s"],
            [[method, proof, g[0], f"{g[1]:.1f}", g[2], f"{g[3]:.1f}"]
             for (method, proof), g in sorted(groups.items())]),
        section(
            "The dearest games of the c8g chain",
            "The ten completed games with the most search seconds.",
            ["n", "proof", "nodes", "search s", "total s"],
            [[r["n"], r["proof"], r["nodes"], f"{r['seconds']['search']:.1f}", f"{r['seconds']['total']:.2f}"]
             for r in dearest]),
    ])


TABLES = {"phases": phases, "zero": zero_node_games, "setup": setup, "frames": frames, "fixed": fixed_model,
          "check": check_model, "nodes": node_costs, "widths": widths, "start": start_search, "gaps": gaps,
          "series": series, "grow": grow_runs, "c8g": c8g_chain}


def main(args):
    global chain_end
    names = list(args)
    if "--chain-end" in names:
        i = names.index("--chain-end")
        if i + 1 == len(names):
            print("--chain-end needs the last game of the committed chain", file=sys.stderr)
            return 2
        chain_end = int(names[i + 1])
        del names[i : i + 2]
    unknown = [name for name in names if name not in TABLES]
    if unknown:
        print(f"unknown table {', '.join(unknown)}; the tables are {', '.join(TABLES)}", file=sys.stderr)
        return 2
    print("\n\n".join(TABLES[name]() for name in names or TABLES))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
