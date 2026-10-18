"""Loading the data of the landscape study: the files of this directory and the committed chain."""
import gzip
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
CHAIN = HERE.parents[2] / "results" / "strict.jsonl"


def lines(path):
    """The JSON objects of a file of lines (gzipped if its name ends in .gz)."""
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt") as f:
        return [json.loads(line) for line in f if line.strip()]


def by_n(path):
    """The lines of a file by their game."""
    return {d["n"]: d for d in lines(path)}


def quantile(xs, p):
    """The p-quantile of xs by position in the sorted list (p10 is the element a tenth of the way up)."""
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(p * len(xs)))]
