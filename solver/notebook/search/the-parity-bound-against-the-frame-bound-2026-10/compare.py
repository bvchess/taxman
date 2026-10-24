#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The parity bound against the frame bound: the tables of the notebook entry beside this file.

    ./compare.py [--lists FRAME PARITY]

Reads `frame_2_8191.jsonl.gz` (from `framebound.rs`) and `parity_2_8191.jsonl.gz` (from `paritysplit.rs`) beside
this file, joins them by n, and prints, in Markdown, every board where the parity bound differs from ΣU* plus the frame
bound, and a table by band of the checks.

With `--lists`, FRAME and PARITY are the same runs made with their lists (`lists`, `--lists yes`), and it also writes:

* `extras.jsonl.gz`: for every board whose GF(2) candidates outnumber its candidates, the extra numbers, and those of
  them in the heaviest set the prover found;
* `margins_in.txt`: every fiftieth of those boards, with all its extra numbers, and `margins_near_in.txt`: in each
  hundred of n, the board whose largest extra number y has the largest ratio y/(n/2), with that number; the inputs of
  `paritymargin.rs`.

If `margins.jsonl` and `margins_near.jsonl` (the outputs of `paritymargin.rs`) are there, it prints their losses.
"""
import gzip
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MARGINS = (("margins.jsonl", "every fiftieth board, each extra number"),
           ("margins_near.jsonl", "in each hundred of n, the board whose largest extra number y has the largest "
                                  "y/(n/2), that number"))


def load(path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt") as f:
        return {d["n"]: d for d in map(json.loads, f)}


def checks(frame, parity, boards):
    gaps, bands = [], {}
    for n in boards:
        f, p = frame[n], parity[n]
        assert f["in_the_bag"] == p["ustar_sum"], f"n={n}: the two programs' ΣU* differ"
        gap = p["parity"] - f["in_the_bag"] - f["frame"]
        if gap:
            gaps.append((n, gap))
        same = (f["candidates"], f["cand_sum"], f["cand_digest"]) == (p["free"], p["free_sum"], p["free_digest"])
        # The candidates are GF(2) candidates (B6), so equal counts mean equal sets; the digest checks it.
        assert same or p["free"] > f["candidates"], f"n={n}: fewer GF(2) candidates than candidates"
        b = bands.setdefault((n - 1) // 1000, {"boards": 0, "equal": 0, "trivial": 0, "limit": 0, "sources": 0,
                                               "diamond": 0, "extra_boards": 0, "extra": 0})
        b["boards"] += 1
        b["equal"] += gap == 0
        b["trivial"] += f["candidates"] == 0
        b["limit"] += f["frame"] != f["frame_free"]
        b["sources"] += p["sources_ustar"] == "yes" and p["parity_ustar"] == p["parity"]
        b["diamond"] += p["diamond"] == 0
        b["extra_boards"] += not same
        b["extra"] += p["free"] - f["candidates"]
    return gaps, bands


def print_checks(gaps, bands, boards):
    print(f"## Boards {boards[0]}..{boards[-1]}: {len(boards)}\n")
    if gaps:
        print("| n | parity bound − ΣU* − frame bound |\n|---|---|")
        for n, gap in gaps:
            print(f"| {n} | {gap} |")
    else:
        print("The parity bound equals ΣU* plus the frame bound at every board.")
    print()
    print("| n | boards | equal | nothing to promote | limit lowers the frame bound | sources are U* | (♦) holds | "
          "boards with extra GF(2) candidates | extra numbers |")
    print("|---|---|---|---|---|---|---|---|---|")
    keys = ("boards", "equal", "trivial", "limit", "sources", "diamond", "extra_boards", "extra")
    total = dict.fromkeys(keys, 0)
    for band, b in sorted(bands.items()):
        lo, hi = max(2, band * 1000 + 1), min(boards[-1], band * 1000 + 1000)
        print(f"| {lo}-{hi} | " + " | ".join(str(b[k]) for k in keys) + " |")
        for k in keys:
            total[k] += b[k]
    print("| all | " + " | ".join(str(total[k]) for k in keys) + " |")


def write_lists(boards, frame_lists, parity_lists):
    used_boards, found = 0, []
    with gzip.open(HERE / "extras.jsonl.gz", "wt") as out:
        for n in boards:
            cands = set(frame_lists[n]["cand_list"])
            free = set(parity_lists[n]["free_list"])
            assert cands <= free, f"n={n}: a candidate is a loop"
            extra = sorted(free - cands)
            if extra:
                used = sorted(set(parity_lists[n]["chosen"]) & set(extra))
                used_boards += bool(used)
                out.write(json.dumps({"n": n, "extra": extra, "in_heaviest_set": used}) + "\n")
                found.append((n, extra))
    with open(HERE / "margins_in.txt", "w") as out:
        for n, extra in found[::50]:
            out.write(f"{n} {' '.join(map(str, extra))}\n")
    nearest = {}
    for n, extra in found:
        best = nearest.get(n // 100)
        if best is None or extra[-1] / (n // 2) > best[1][-1] / (best[0] // 2):
            nearest[n // 100] = (n, extra)
    with open(HERE / "margins_near_in.txt", "w") as out:
        for n, extra in sorted(nearest.values()):
            out.write(f"{n} {extra[-1]}\n")
    print(f"\nBoards whose heaviest parity set holds an extra GF(2) candidate: {used_boards}")


def print_margins():
    for name, what in MARGINS:
        if not (HERE / name).exists():
            continue
        margins = load(HERE / name)
        print(f"\n### {name}: {what}\n")
        print("| n | boards | numbers tried | least loss | least of loss − (n/2 − y) | most of loss − (n/2 − y) |")
        print("|---|---|---|---|---|---|")
        rows = {}
        for n, d in sorted(margins.items()):
            r = rows.setdefault((n - 1) // 1000, {"boards": 0, "tried": 0, "least": [], "excess": []})
            r["boards"] += 1
            for y, loss in d["margins"]:
                r["tried"] += 1
                r["least"].append((loss, n, y))
                r["excess"].append(loss - (n // 2 - y))
        for band, r in sorted(rows.items()):
            loss, n, y = min(r["least"])
            print(f"| {band * 1000 + 1}-{band * 1000 + 1000} | {r['boards']} | {r['tried']} | {loss} (n = {n}, "
                  f"y = {y}) | {min(r['excess'])} | {max(r['excess'])} |")


def main():
    frame = load(HERE / "frame_2_8191.jsonl.gz")
    parity = load(HERE / "parity_2_8191.jsonl.gz")
    assert frame.keys() == parity.keys(), "the two runs cover the same boards"
    boards = sorted(frame)
    print_checks(*checks(frame, parity, boards), boards)
    if len(sys.argv) == 4 and sys.argv[1] == "--lists":
        write_lists(boards, load(sys.argv[2]), load(sys.argv[3]))
    print_margins()


if __name__ == "__main__":
    main()
