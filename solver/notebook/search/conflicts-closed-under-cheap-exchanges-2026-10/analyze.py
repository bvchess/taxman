#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The tables of the entry from the lines and files of `wall images` (solver/frame/examples/wall/images.rs, whose module
comment defines their fields).

usage: python3 -I analyze.py RUNS.jsonl DIR...

RUNS.jsonl holds the lines of `wall images`; the directories hold the rows of the test (ID.images.tsv) and the classes
of the learning (ID.classes.tsv).  A line with a field `run` made the classes and is left out of the tables.  A unit's
tables are made from the files found.
"""
import collections
import json
import statistics
import sys
from pathlib import Path

CHEAP = 50  # an exclusion priced under this is cheap (the anatomy's line)
PRICES = [(-10**9, 10), (10, 25), (25, 50), (50, 100), (100, 200), (200, 10**9)]
SIZES = [(1, 2), (2, 3), (3, 6), (6, 10**9)]


def read_rows(path):
    with open(path) as f:
        header = f.readline().rstrip("\n").split("\t")
        for line in f:
            row = dict(zip(header, line.rstrip("\n").split("\t")))
            for key in ("start", "end", "d", "x", "home", "p", "q", "m", "b", "m2", "nk", "moved", "imin", "nc"):
                row[key] = int(row[key])
            row["price"] = None if row["price"] == "-" else int(row["price"])
            row["img"] = None if row["img"] == "-" else row["img"] == "1"
            yield row


def union(rows):
    """The steps under any of the rows' exclusions: the union of their intervals [start, end]."""
    total, reach = 0, 0
    for start, end in sorted((r["start"], r["end"]) for r in rows):
        if end <= reach:
            continue
        total += end - max(start, reach + 1) + 1
        reach = end
    return total


def pct(a, b):
    return f"{100 * a / b:.1f}%" if b else "-"


def bucket(value, buckets):
    """The bucket [lo, hi) that holds value."""
    return next((lo, hi) for lo, hi in buckets if lo <= value < hi)


def label(lo, hi):
    if lo < 0:
        return f"under {hi}"
    if hi >= 10**9:
        return f"{lo}+"
    return f"{lo}" if hi == lo + 1 else f"{lo}-{hi - 1}"


def table(title, head, lines):
    print(f"\n{title}\n")
    print("| " + " | ".join(head) + " |")
    print("|" + "---|" * len(head))
    for line in lines:
        print("| " + " | ".join(str(c) for c in line) + " |")


def analyze(uid, line, rows):
    steps = line["nodes"]
    priced = [r for r in rows if r["price"] is not None]
    swaps = [r for r in priced if r["kind"] in "TP"]
    images = [r for r in swaps if r["img"]]
    real = [r for r in swaps if r["nk"] >= 2]  # the conflict has two members or more
    real_images = [r for r in real if r["img"]]
    cheap = [r for r in priced if r["price"] < CHEAP]
    cheap_swaps = [r for r in swaps if r["price"] < CHEAP]
    print(f"\n## {uid}: {steps:,} steps, {line['exclusions']:,} exclusions, {len(priced):,} priced, "
          f"cpu {line['cpu_s']:.0f} s, best {line['best']} (opt {line['opt']})")

    def row(name, sel, sel_images):
        return [name, f"{len(sel):,}", pct(len(sel), len(priced)), f"{len(sel_images):,}", pct(len(sel_images), len(sel)),
                f"{union(sel):,}", pct(union(sel), steps), f"{union(sel_images):,}", pct(union(sel_images), steps)]

    head = ["exclusions", "count", "of priced", "image a conflict", "share", "steps under", "of steps",
            "steps under those with a conflict image", "of steps"]
    kinds = []
    for name, kind in [("twin swaps", "T"), ("paired exchanges", "P")]:
        sel = [r for r in swaps if r["kind"] == kind]
        kinds.append(row(name, sel, [r for r in sel if r["img"]]))
    kinds.append(row("both", swaps, images))
    kinds.append(row("both, conflict of 2+", real, real_images))
    kinds.append(row("both, cheap", cheap_swaps, [r for r in cheap_swaps if r["img"]]))
    cheap_real = [r for r in cheap_swaps if r["nk"] >= 2]
    kinds.append(row("both, cheap, conflict of 2+", cheap_real, [r for r in cheap_real if r["img"]]))
    going = [r for r in real if r["end"] > r["start"]]
    kinds.append(row("both, conflict of 2+, the node goes on", going, [r for r in going if r["img"]]))
    cheap_going = [r for r in going if r["price"] < CHEAP]
    kinds.append(row("both, cheap, conflict of 2+, the node goes on", cheap_going, [r for r in cheap_going if r["img"]]))
    others = [r for r in priced if r["kind"] == "O"]
    kinds.append(["other substitutions", f"{len(others):,}", pct(len(others), len(priced)), "-", "-",
                  f"{union(others):,}", pct(union(others), steps), "-", "-"])
    kinds.append(["all priced", f"{len(priced):,}", "100%", "-", "-", f"{union(priced):,}", pct(union(priced), steps),
                  "-", "-"])
    kinds.append(["cheap (all substitutions)", f"{len(cheap):,}", pct(len(cheap), len(priced)), "-", "-",
                  f"{union(cheap):,}", pct(union(cheap), steps), "-", "-"])
    table("By substitution", head, kinds)

    lines = []
    for lo, hi in SIZES:
        sel = [r for r in swaps if bucket(r["nk"], SIZES) == (lo, hi)]
        im = [r for r in sel if r["img"]]
        both_moved = [r for r in sel if r["moved"] == 2]
        lines.append([label(lo, hi), f"{len(sel):,}", f"{len(im):,}", pct(len(im), len(sel)),
                      f"{len(both_moved):,}", f"{union(im):,}"])
    table("Twin swaps and paired exchanges by the size of the conflict K",
          ["size of K", "exclusions", "image a conflict", "share", "K holds both numbers lost", "steps under (images)"],
          lines)

    lines = []
    for lo, hi in PRICES:
        sel = [r for r in swaps if bucket(r["price"], PRICES) == (lo, hi)]
        im = [r for r in sel if r["img"]]
        sel2 = [r for r in sel if r["nk"] >= 2]
        im2 = [r for r in sel2 if r["img"]]
        lines.append([label(lo, hi), f"{len(sel):,}", pct(len(im), len(sel)), f"{len(sel2):,}", pct(len(im2), len(sel2)),
                      f"{union(im2):,}"])
    table("By price", ["price", "exclusions", "image a conflict", "with K of 2+", "image a conflict",
                       "steps under (images, K of 2+)"], lines)

    lines = []
    for home in sorted({r["home"] for r in swaps}):
        sel = [r for r in swaps if r["home"] == home]
        sel2 = [r for r in sel if r["nk"] >= 2]
        im2 = [r for r in sel2 if r["img"]]
        lines.append([home, f"{len(sel):,}", pct(sum(r['img'] for r in sel), len(sel)), f"{len(sel2):,}",
                      pct(len(im2), len(sel2)), f"{union(im2):,}"])
    table("By the member's home", ["home", "exclusions", "image a conflict", "with K of 2+", "image a conflict",
                                   "steps under (images, K of 2+)"], lines)

    pairs = collections.defaultdict(list)
    for r in swaps:
        pairs[(min(r["p"], r["q"]), max(r["p"], r["q"]), r["kind"])].append(r)
    lines = []
    for (p, q, kind), sel in sorted(pairs.items(), key=lambda kv: -len(kv[1]))[:15]:
        sel2 = [r for r in sel if r["nk"] >= 2]
        lines.append([f"{p} ↔ {q}", kind, f"{len(sel):,}", statistics.median(r["price"] for r in sel),
                      pct(sum(r["img"] for r in sel), len(sel)), f"{len(sel2):,}", pct(sum(r["img"] for r in sel2), len(sel2))])
    table("The prime pairs met most", ["primes", "kind", "exclusions", "median price", "image a conflict",
                                       "with K of 2+", "image a conflict"], lines)

    members = collections.defaultdict(list)
    for r in swaps:
        members[(r["x"], r["lost"], r["gained"])].append(r)
    lines = []
    for (x, lost, gained), sel in sorted(members.items(), key=lambda kv: -len(kv[1]))[:12]:
        lines.append([x, lost, gained, sel[0]["kind"], f"{len(sel):,}", statistics.median(r["price"] for r in sel),
                      statistics.median(r["nk"] for r in sel), pct(sum(r["img"] for r in sel), len(sel))])
    table("The substitutions met most", ["member", "lost", "gained", "kind", "times", "median price", "median size of K",
                                         "image a conflict"], lines)

    nexts = collections.Counter((r["next"], r["nc"]) for r in real_images)
    lines = [[n, "computed" if c else "known", f"{k:,}", pct(k, len(real_images))] for (n, c), k in nexts.most_common()]
    table("Where the image of a conflict of 2+ is a conflict: the node's next conflict",
          ["next conflict", "found", "times", "share"], lines)
    minimal = collections.Counter("same size" if r["imin"] == r["nk"] else "smaller" for r in real_images)
    print("\nminimal conflict within the image, against K:", dict(minimal))


def read_classes(path):
    with open(path) as f:
        header = f.readline().rstrip("\n").split("\t")
        for line in f:
            row = dict(zip(header, line.rstrip("\n").split("\t")))
            for key in header[1:]:
                row[key] = int(row[key])
            row["key"] = [int(x) for x in row["key"].split(",")]
            yield row


def learning(lines):
    """Step 2: the search that learns the images against the search alone."""
    out = []
    for uid in sorted({uid for uid, _ in lines}):
        check, bare = lines.get((uid, "check")), lines.get((uid, "bare"))
        if not (check and bare):
            continue
        assert check["nodes"] == bare["nodes"] and check["best"] == bare["best"] == bare["opt"], uid
        steps, leaves, cpu = bare["nodes"], check["leaves"], bare["cpu_s"]
        out.append([uid, "alone", f"{steps:,}", "", f"{leaves:,}", "", f"{cpu:.0f}", "", "", "", ""])
        for mode in ("learn", "closure"):
            learn = lines.get((uid, mode))
            if not learn:
                continue
            assert learn["completed"] and learn["best"] == learn["opt"], (uid, mode)
            out.append(["", mode, f"{learn['nodes']:,}", f"{learn['nodes'] / steps - 1:+.1%}", f"{learn['leaves']:,}",
                        f"{learn['leaves'] / leaves - 1:+.1%}", f"{learn['cpu_s']:.0f}", f"{learn['cpu_s'] / cpu - 1:+.1%}",
                        f"{learn['conflicts']:,}", f"{learn['proposed']:,}", f"{learn['learned']:,}"])
    table("Learning the images: the search alone (`--bare`, its leaves from the test's run), learning the images "
          "(`--learn`) and their closure (`--learn --closure`)",
          ["unit", "search", "steps", "change", "leaves", "change", "CPU s", "change", "conflicts computed",
           "images proposed", "conflicts learned in them"], out)


def classes(uid, rows):
    """The classes of the learning: a conflict computed, with the conflicts learned within its images."""
    uses = sorted((r["branched"] + r["ruled"] for r in rows), reverse=True)
    total = sum(uses)

    def holding(share):
        run = 0
        for i, u in enumerate(uses):
            run += u
            if run >= share * total:
                return i + 1
        return len(uses)

    def spread(key):
        counts = collections.Counter(min(r[key], 6) for r in rows)
        return ", ".join(f"{'6+' if s == 6 else s}: {k:,}" for s, k in sorted(counts.items()))

    print(f"\n### Classes of {uid}\n")
    print(f"* {len(rows):,} conflicts computed, {sum(r['times'] for r in rows):,} times "
          f"({sum(r['times'] > 1 for r in rows):,} more than once, at most {max(r['times'] for r in rows)} times); "
          f"{sum(r['learned'] > 0 for r in rows):,} with a conflict learned in an image")
    print(f"* size of the conflict computed: {spread('size')}")
    print(f"* distinct nogoods learned in its images: {spread('distinct')}")
    print(f"* uses: {sum(r['branched'] for r in rows):,} steps branched on a conflict an image's nogood gave, "
          f"{sum(r['ruled'] for r in rows):,} candidates such a nogood left alone in a child (an estimate of its "
          f"rule-outs)")
    if total:
        print(f"* the class used most has {uses[0] / total:.1%} of the uses, the 10 most {sum(uses[:10]) / total:.1%}; "
              f"{holding(0.5):,} classes have half the uses and {holding(0.8):,} four fifths; {sum(u == 0 for u in uses):,} "
              f"have none")


def main():
    runs, dirs = sys.argv[1], [Path(d) for d in sys.argv[2:]]
    lines = {}
    with open(runs) as f:
        for text in f:
            if text.strip():
                line = json.loads(text)
                if "run" not in line:
                    lines[(line["id"], line["mode"])] = line

    def find(name):
        return next((d / name for d in dirs if (d / name).exists()), None)

    units = sorted({uid for uid, _ in lines})
    for uid in units:
        path = find(f"{uid}.images.tsv")
        if path and (uid, "check") in lines:
            analyze(uid, lines[(uid, "check")], list(read_rows(path)))
    learning(lines)
    for uid in units:
        path = find(f"{uid}.classes.tsv")
        if path:
            classes(uid, list(read_classes(path)))


if __name__ == "__main__":
    main()
