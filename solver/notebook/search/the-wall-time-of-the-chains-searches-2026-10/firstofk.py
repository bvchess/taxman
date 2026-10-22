#!/usr/bin/env python3
"""The time for the first of k independent finders to reach a game's best, from runs of one finder each.

Usage: firstofk.py RUNS...

RUNS are the jsonl lines of `examples/finder.rs` (one random seed a run, `--target` the best, and a budget in
`--seconds`).  A run that reached the target took `found_s`; one that did not is censored at its budget.  For each
game and k = 1, 2, 4, 8, 16 (up to the number of runs), prints the mean over every k-subset of the runs of the time the
first of them reached the best (a censored run counts its budget, so the mean is a lower bound where one is in the
subset), and how many subsets reached it at all.
"""
import itertools
import json
import sys


def main():
    games = {}
    for path in sys.argv[1:]:
        for line in open(path):
            d = json.loads(line)
            if "random" not in d:
                continue
            key = (d["game"], d["start"])
            games.setdefault(key, []).append((d["found_s"] if d["reached"] else None, d["seconds"], d["random"]))
    for (game, start), runs in sorted(games.items()):
        times = [t for t, _, _ in runs]
        print(f"game {game} from {start}: {len(runs)} runs; time to the best: "
              + ", ".join("—" if t is None else f"{t:.0f}" for t in sorted(times, key=lambda t: (t is None, t))))
        for k in (1, 2, 4, 8, 16):
            if k > len(runs):
                break
            firsts, reached = [], 0
            for subset in itertools.combinations(runs, k):
                done = [t for t, _, _ in subset if t is not None]
                if done:
                    reached += 1
                    firsts.append(min(done))
                else:
                    firsts.append(max(b for _, b, _ in subset))
            mean = sum(firsts) / len(firsts)
            total = len(firsts)
            print(f"  k={k:2d}: mean first time {mean:6.0f} s; subsets that reached the best {reached}/{total}")


if __name__ == "__main__":
    main()
