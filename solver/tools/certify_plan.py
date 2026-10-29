#!/usr/bin/env python3
"""The work list and the cost estimate of the certificates campaign (`taxman certify` over a range of games).

Standard library only; every path comes from the arguments, so it runs from anywhere.

A certificate proves, on the whole board, a game whose recorded score is optimal only if the frame model is right: the
prover's barred bounds (`upper_*.jsonl`) say which games need one.  A campaign on a cloud machine certifies them all.
This script makes the campaign's work list and the cost estimate that the owner approves, from the data files alone.

Inputs, all JSON lines, and a file whose name ends in .gz is read through gzip, every member of it (the flags that
take several files read them in order):
  --results   the chain's results files (the format of results/optimal.jsonl.gz).  Completed lines only; a later line
              for a board replaces an earlier one.  Fields used: n, score, proof (exhausted, cap, same_search,
              prime_formula, trivial, known), nodes, threads, seconds.search, same_search.n (the game whose search a
              repeat repeats) and known.source (where a known game rests: bounds, the bounds store; strict, the
              committed chain; or optimal, the table of optimal games).
  --upper     the prover's barred bounds.  Fields: n, score, relation (prime; chain, which is a score at its ceiling
              n + score(n-1); none), needs (1 if the game needs a certificate), open (where it needs one: a list of
              [u, omega, margin], so |O| is its length).  A board in two files must have equal lines, but for the
              timing `secs`.  The score of a board must be the results' score.  Every board of the range needs a line.
  --problems  (optional) one line per game that needs a certificate, made by `examples/extended.rs problems`: n, score,
              open (|O|, a count), rejected (|R|), frames, two_sided, one_sided, in_the_bag, worth, words, width,
              digest, same_as (a board m in n-8..n-1 whose extended problem equals n's, or null).  Without it the
              shares are unknown and |R| is null.
  --gaps      (optional) lines of solver/notebook/search/the-gap-past-2175-2026-10/gaps_chain.jsonl: n and gap, the root
              gap of the game's search problem.

The cost of a plain search.  The chain ran several searches at once (a window), so a game's recorded seconds.search is
the time it shared the machine, not what it cost.  The cost is taken from nodes instead.  Each --era A-B:HOURS says that
the chain's run over the boards A..B took HOURS of wall time on one machine of --machine-threads threads; the era's c
is HOURS x 3600 x threads / the sum of nodes over the completed lines of A..B (every proof kind), thread-seconds per
node.  A plain search of `nodes` nodes on a board of the era costs nodes x c / 3600 thread-hours, and a certificate
costs --overhead times that.  The report prints each era's c, its nodes and its recorded thread-hours beside the wall
time, so the window's inflation is visible.  Every board of the range must fall in an era.

Per game, for every board of the range (plan.jsonl has one line each, null where a field does not apply):
  relation    prime, ceiling (upper's chain) or none; needs, open (|O|) from upper; rejected (|R|) from problems
  proof       the results' proof of a game that needs a certificate, and for a searched game (exhausted) chain_nodes,
              chain_seconds (seconds.search) and chain_threads; for a repeat (same_search) m, the game it repeats,
              source, the game that m comes down to when same_search is followed to a proof that is not one, and its
              source_proof and source_exhausted
  share       the board m when the problems line says same_as = m, m is a planned game that needs a certificate too, and
              the two lines' worth are equal: taxman certify settles n from m's certificate with no search.  When
              same_as is set, m is such a game and the worths differ, the share is refused (share_refused) and the
              game is listed in the report: it should not exist
  basis       how the expected cost is known: shared (no cost), chain (its own search was exhausted: the cost from its
              nodes), source (a repeat whose source was exhausted: from the source's nodes), gap model (a repeat whose
              source was proven at its ceiling, cap, so no search of the problem has finished: the nodes that the cost
              model of the research agenda, section 2, predicts from the source's root gap G, 10^(2.30 + 0.0479 G /
              n^0.3 + 0.06) for the board n of the game itself, the last term being the window and the threads of the
              chain), unknown (an input is missing, or its proof has no search to price; the reason is recorded, the
              cost is null and the report lists the game), or none (no certificate needed, cost 0)
  cost_nodes  the nodes that the cost is from, with cost_seconds and cost_threads, the recorded seconds.search and
              threads of the game or its source (null for a gap-model game)
  expected_thread_hours   overhead x the plain cost of cost_nodes, in thread-hours
  expected_thread_hours_recorded   the same from the recorded seconds x threads (the upper end, which ignores the
              window; a gap-model game as above)

A known game (proof known) rests on its source, known.source, not on a search of its own: the chain took its game from
the source (nodes 0), or ran a search that the bounds store's upper bound stopped before it finished.  No search of its
problem was exhausted, so there is nothing to price it by.  It is planned as any game is: relation, needs, open,
rejected and share come from the other files, and it costs 0 where it needs no certificate or shares one.  One that
needs a certificate and shares none stops the plan with an error that names the board, since a cost would be a guess.

The lists (lists/*.txt in --out-dir; one n a line after the header lines that start with `# `).  A group is a
game that has no share (its head) with the games that share with it, directly or through a chain of shares: it goes
into one list, in increasing n, since a share is settled only from a certificate in the same output file.  pilot.txt
holds the groups of the five dearest heads, dear.txt those of the next fifteen, and rest_1.txt .. rest_K.txt the other
groups, taken in increasing n of their heads, each to the list with the smallest expected total so far.  Every game
that needs a certificate is in exactly one list.

plan.md (also printed): the inputs and eras, a table by band of 100 boards with the totals of the two parts (up to 3314
and from 3315, when the range has both) and of all, the twenty dearest games, the boards 3424, 3425 and 3440 if in the
range, the estimate (thread-hours, machine-hours, dollars on demand and at spot, the floor on wall time, the lists'
totals, the total from recorded seconds), and the games of unknown cost and with refused shares.

usage: certify_plan.py --results FILE... --upper FILE... --era A-B:HOURS [--era ...] [--problems FILE...]
           [--gaps FILE...] [--from A] [--to B] [--machine-threads T] [--overhead X] [--price-on-demand D]
           [--price-spot S] [--lists K] [--out-dir DIR]
The exit status is 1 if an input is wrong (the message names the file and line, or the board), or if a known game
needs a certificate and shares none (the message names the board).
"""
import argparse
import gzip
import json
import math
import re
import shlex
import sys
import zlib
from dataclasses import dataclass
from pathlib import Path
from typing import NamedTuple, Optional

FIRST_BOARD = 2176  # the default start of the range
MACHINE_THREADS = 64  # the c8g.16xlarge
OVERHEAD = 2.0  # a certificate costs this many plain searches of its problem
PRICE_ON_DEMAND = 2.5517  # dollars an hour, c8g.16xlarge in us-east-1
PRICE_SPOT = 0.6752  # dollars an hour, the spot price paid on 2026-10-15
LISTS = 4  # balanced lists for the rest
PILOT_HEADS = 5  # the dearest heads of the pilot list
DEAR_HEADS = 20  # the dearest heads of the pilot and dear lists together
DEAREST_SHOWN = 20  # games in the report's table of the dearest
BAND = 100  # boards in a band of the report's table
PART_END = 3314  # the first part of the table ends here, the second starts after it
SPOTLIGHT = (3424, 3425, 3440)  # boards the report names
# The cost model of the research agenda (solver/docs/AGENDA.md, "The diagnosis of the wall"): log10 nodes = a + b G /
# n^p, plus the window.
GAP_INTERCEPT, GAP_SLOPE, GAP_POWER, WINDOW_DECADES = 2.30, 0.0479, 0.3, 0.06
PROOFS = {"trivial", "exhausted", "cap", "prime_formula", "same_search", "known"}
RELATIONS = {"prime": "prime", "chain": "ceiling", "none": "none"}  # upper's word -> the plan's
INT = int
NUMBER = (int, float)
KIND_NAMES = {INT: "an integer", NUMBER: "a number", str: "a string", list: "a list", dict: "an object"}
ERA = re.compile(r"(\d+)-(\d+):(\d+(?:\.\d*)?|\.\d+)")


class PlanError(Exception):
    """An input is wrong; the message names the file and line, or the board."""


class Result(NamedTuple):
    """The fields of a completed results line that the plan uses."""

    n: int
    score: int
    proof: str
    nodes: int
    threads: int
    seconds: float  # seconds.search
    repeat_of: Optional[int]  # same_search.n of a repeat
    known_source: Optional[str] = None  # known.source of a known game: bounds, strict or optimal


class Era(NamedTuple):
    """A stretch of boards that the chain ran in `hours` of wall time, with what the results say of it."""

    lo: int
    hi: int
    hours: float
    games: int = 0  # completed games of the results in lo..hi
    nodes: int = 0
    recorded: float = 0.0  # thread-hours: the sum of seconds.search x threads / 3600
    c: float = 0.0  # thread-seconds per node


@dataclass
class Plan:
    """Everything the report and the files are made from."""

    command: str
    files: dict  # flag -> the paths given
    lo: int
    hi: int
    eras: list
    machine_threads: int
    overhead: float
    price_on_demand: float
    price_spot: float
    games: list  # one dict per board of the range, in order
    lists: list  # one dict per list, in order


# --- reading the inputs ---------------------------------------------------------------------------------------------


def read_lines(path):
    """The lines of a JSON-lines file as (line number, object); a file whose name ends in .gz is read through gzip,
    every member of it.  A line that is not a JSON object is an error."""
    try:
        if str(path).endswith(".gz"):
            with gzip.open(path, "rt", encoding="utf-8") as f:
                text = f.read()
        else:
            text = Path(path).read_text()
    except (OSError, EOFError, zlib.error, UnicodeDecodeError) as e:
        raise PlanError(f"{path}: cannot read: {getattr(e, 'strerror', None) or e}") from e
    for number, line in enumerate(text.split("\n"), 1):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as e:
            raise PlanError(f"{path} line {number}: not JSON: {e}") from e
        if not isinstance(row, dict):
            raise PlanError(f"{path} line {number}: not a JSON object")
        yield number, row


def need(row, key, kind, where, low=None):
    """row[key], which must be present and of the kind (an int or a finite number; a bool is neither)."""
    if key not in row:
        raise PlanError(f"{where}: no field {key!r}")
    value = row[key]
    numeric = kind in (INT, NUMBER)
    if (numeric and isinstance(value, bool)) or not isinstance(value, kind):
        raise PlanError(f"{where}: field {key!r} is {value!r}, not {KIND_NAMES[kind]}")
    if isinstance(value, float) and not math.isfinite(value):
        raise PlanError(f"{where}: field {key!r} is {value!r}")
    if low is not None and value < low:
        raise PlanError(f"{where}: field {key!r} is {value!r}, below {low}")
    return value


def load_results(paths):
    """{n: Result} of the completed lines of the files; a later line for a board replaces an earlier one."""
    games = {}
    for path in paths:
        for number, row in read_lines(path):
            if row.get("completed") is not True:
                continue
            n = need(row, "n", INT, f"{path} line {number}", low=1)
            where = f"{path} line {number} (board {n})"
            proof = need(row, "proof", str, where)
            if proof not in PROOFS:
                raise PlanError(f"{where}: unknown proof {proof!r}")
            repeat_of = None
            if proof == "same_search":
                repeat_of = need(need(row, "same_search", dict, where), "n", INT, f"{where}, same_search", low=1)
                if repeat_of >= n:
                    raise PlanError(f"{where}: repeats the search of board {repeat_of}, which is not below it")
            known_source = None
            if proof == "known":
                known_source = need(need(row, "known", dict, where), "source", str, f"{where}, known")
            games[n] = Result(
                n,
                need(row, "score", INT, where),
                proof,
                need(row, "nodes", INT, where, low=0),
                need(row, "threads", INT, where, low=1),
                need(need(row, "seconds", dict, where), "search", NUMBER, f"{where}, seconds", low=0),
                repeat_of,
                known_source,
            )
    return games


def same_but_timing(a, b):
    """Whether two lines of a prover file agree; `secs`, how long the prover took, is no part of the fact."""
    return {k: v for k, v in a.items() if k != "secs"} == {k: v for k, v in b.items() if k != "secs"}


def load_upper(paths):
    """{n: {score, relation, needs, open, where}} of the barred bounds; a board in two lines must have equal lines."""
    seen = {}  # n -> (where, row)
    games = {}
    for path in paths:
        for number, row in read_lines(path):
            n = need(row, "n", INT, f"{path} line {number}", low=1)
            where = f"{path} line {number} (board {n})"
            if n in seen:
                if not same_but_timing(seen[n][1], row):
                    raise PlanError(f"{where}: differs from {seen[n][0]}")
                continue
            seen[n] = (where, row)
            relation = need(row, "relation", str, where)
            if relation not in RELATIONS:
                raise PlanError(f"{where}: unknown relation {relation!r}")
            needs = need(row, "needs", INT, where)
            if needs not in (0, 1):
                raise PlanError(f"{where}: field 'needs' is {needs!r}, not 0 or 1")
            games[n] = {
                "score": need(row, "score", INT, where),
                "relation": RELATIONS[relation],
                "needs": needs,
                "open": len(need(row, "open", list, where)) if needs == 1 else None,
                "where": where,
            }
    return games


def load_problems(paths):
    """{n: {open, rejected, worth, same_as, score, where}} of the extended problems; a board in two lines must agree."""
    seen = {}
    games = {}
    for path in paths:
        for number, row in read_lines(path):
            n = need(row, "n", INT, f"{path} line {number}", low=1)
            where = f"{path} line {number} (board {n})"
            if n in seen:
                if row != seen[n][1]:
                    raise PlanError(f"{where}: differs from {seen[n][0]}")
                continue
            seen[n] = (where, row)
            for key in ("worth", "same_as"):
                if key not in row:
                    raise PlanError(f"{where}: no field {key!r}")
            same_as = row["same_as"]
            if same_as is not None:
                need(row, "same_as", INT, where, low=1)
                if same_as >= n:
                    raise PlanError(f"{where}: same_as {same_as} is not below the board")
            games[n] = {
                "rejected": need(row, "rejected", INT, where, low=0),
                "worth": row["worth"],
                "same_as": same_as,
                "open": need(row, "open", INT, where, low=0) if "open" in row else None,
                "score": need(row, "score", INT, where) if "score" in row else None,
                "where": where,
            }
    return games


def load_gaps(paths):
    """{n: root gap}; a board in two lines must have the same gap."""
    gaps = {}
    for path in paths:
        for number, row in read_lines(path):
            n = need(row, "n", INT, f"{path} line {number}", low=1)
            where = f"{path} line {number} (board {n})"
            gap = need(row, "gap", NUMBER, where, low=0)
            if n in gaps and gaps[n] != gap:
                raise PlanError(f"{where}: gap {gap}, but an earlier line has {gaps[n]}")
            gaps[n] = gap
    return gaps


# --- the eras --------------------------------------------------------------------------------------------------------


def parse_era(text):
    """An --era A-B:HOURS argument as an Era."""
    m = ERA.fullmatch(text)
    if not m:
        raise PlanError(f"--era {text!r}: expected A-B:HOURS, such as 2176-3310:6.4")
    lo, hi, hours = int(m[1]), int(m[2]), float(m[3])
    if lo > hi:
        raise PlanError(f"--era {text!r}: starts after it ends")
    if hours <= 0:
        raise PlanError(f"--era {text!r}: HOURS must be positive")
    return Era(lo, hi, hours)


def calibrate(eras, results, machine_threads):
    """The eras with their completed games, nodes, recorded thread-hours and c (thread-seconds per node) filled in."""
    ordered = sorted(eras)
    for before, after in zip(ordered, ordered[1:]):
        if after.lo <= before.hi:
            raise PlanError(f"--era {before.lo}-{before.hi} and --era {after.lo}-{after.hi} overlap")
    calibrated = []
    for era in ordered:
        inside = [r for n, r in results.items() if era.lo <= n <= era.hi]
        nodes = sum(r.nodes for r in inside)
        if nodes == 0:
            raise PlanError(f"--era {era.lo}-{era.hi}: the results have no nodes in it, so c is undefined")
        recorded = sum(r.seconds * r.threads for r in inside) / 3600
        c = era.hours * 3600 * machine_threads / nodes
        calibrated.append(era._replace(games=len(inside), nodes=nodes, recorded=recorded, c=c))
    return calibrated


def era_of(eras, n):
    """The era that holds the board."""
    return next(era for era in eras if era.lo <= n <= era.hi)


def plain_cost(eras, n, nodes):
    """Thread-hours of a plain search of `nodes` nodes on board n."""
    return nodes * era_of(eras, n).c / 3600


def gap_model_nodes(n, gap):
    """The nodes that the cost model predicts for a board n of root gap `gap`, the chain's window included."""
    return 10 ** (GAP_INTERCEPT + GAP_SLOPE * gap / n**GAP_POWER + WINDOW_DECADES)


# --- the plan --------------------------------------------------------------------------------------------------------


def find_source(results, n):
    """(the game that board n's repeat comes down to, None), or (None, the board the results lack)."""
    m = results[n].repeat_of
    while True:
        r = results.get(m)
        if r is None:
            return None, m
        if r.proof != "same_search":
            return m, None
        m = r.repeat_of


def check_inputs(lo, hi, upper, results, problems):
    """Errors for a board of the range without an upper line, and for lines that contradict each other."""
    missing = [n for n in range(lo, hi + 1) if n not in upper]
    if missing:
        shown = ", ".join(map(str, missing[:8])) + (", ..." if len(missing) > 8 else "")
        raise PlanError(
            f"board {missing[0]} is in the range {lo}..{hi}, but no --upper line has it "
            f"({len(missing)} boards: {shown})"
        )
    for n in range(lo, hi + 1):
        u = upper[n]
        if n in results and results[n].score != u["score"]:
            raise PlanError(f"{u['where']}: score {u['score']}, but the results have {results[n].score}")
        p = problems.get(n)
        if p is None:
            continue
        if u["needs"] != 1:
            raise PlanError(f"{p['where']}: a problem for a game that needs no certificate ({u['where']})")
        if p["score"] is not None and p["score"] != u["score"]:
            raise PlanError(f"{p['where']}: score {p['score']}, but {u['where']} has {u['score']}")
        if p["open"] is not None and p["open"] != u["open"]:
            raise PlanError(f"{p['where']}: open {p['open']}, but {u['where']} has {u['open']} members")


def settle_share(n, lo, hi, upper, problems):
    """(m, refused): the game that board n shares a certificate with, or whether the share is refused."""
    p = problems.get(n)
    if p is None or p["same_as"] is None:
        return None, False
    m = p["same_as"]
    q = problems.get(m)
    if not (lo <= m <= hi and upper[m]["needs"] == 1 and q is not None):
        return None, False
    return (m, False) if q["worth"] == p["worth"] else (None, True)


def blank_record(n, u, rejected):
    """The plan's line for board n before anything is known of its proof: the fields in their order."""
    return {
        "n": n,
        "score": u["score"],
        "relation": u["relation"],
        "needs": u["needs"],
        "open": u["open"],
        "rejected": rejected,
        "proof": None,
        "chain_nodes": None,
        "chain_seconds": None,
        "chain_threads": None,
        "m": None,
        "source": None,
        "source_proof": None,
        "source_exhausted": None,
        "same_as": None,
        "share": None,
        "share_refused": False if u["needs"] == 1 else None,
        "basis": "none",
        "cost_nodes": None,
        "cost_seconds": None,
        "cost_threads": None,
        "gap": None,
        "expected_thread_hours": 0.0,
        "expected_thread_hours_recorded": 0.0,
        "reason": None,
    }


def game_record(n, lo, hi, upper, results, problems, gaps, eras, overhead):
    """The plan's line for board n."""
    p = problems.get(n) if upper[n]["needs"] == 1 else None
    rec = blank_record(n, upper[n], p["rejected"] if p else None)
    if upper[n]["needs"] != 1:
        return rec
    r = results.get(n)
    source = missing = None
    if r is not None:
        rec["proof"] = r.proof
        if r.proof == "exhausted":
            rec.update(chain_nodes=r.nodes, chain_seconds=r.seconds, chain_threads=r.threads)
        elif r.proof == "same_search":
            rec["m"] = r.repeat_of
            source, missing = find_source(results, n)
            if source is not None:
                rec.update(source=source, source_proof=results[source].proof)
                rec["source_exhausted"] = results[source].proof == "exhausted"
    rec["same_as"] = p["same_as"] if p else None
    rec["share"], rec["share_refused"] = settle_share(n, lo, hi, upper, problems)

    def priced(basis, nodes, seconds=None, threads=None, gap=None):
        """Sets the basis and the costs: from the nodes, and from the recorded seconds where there are some."""
        hours = overhead * plain_cost(eras, n, nodes)
        rec.update(basis=basis, cost_nodes=nodes, cost_seconds=seconds, cost_threads=threads, gap=gap)
        rec["expected_thread_hours"] = hours
        rec["expected_thread_hours_recorded"] = hours if seconds is None else overhead * seconds * threads / 3600

    def unknown(reason):
        rec.update(basis="unknown", reason=reason, expected_thread_hours=None, expected_thread_hours_recorded=None)

    if rec["share"] is not None:
        rec["basis"] = "shared"
    elif r is None:
        unknown(f"the results have no completed line for board {n}")
    elif r.proof == "exhausted":
        priced("chain", r.nodes, r.seconds, r.threads)
    elif r.proof == "known":
        raise PlanError(
            f"board {n} needs a certificate, but its proof is known (from {r.known_source}), not a search of its own: "
            "there is no search to price it by"
        )
    elif r.proof != "same_search":
        unknown(f"its proof is {r.proof}, which has no search to price")
    elif source is None:
        unknown(f"it repeats board {r.repeat_of}, whose search comes down to board {missing}, not in the results")
    else:
        s = results[source]
        if s.proof == "exhausted":
            priced("source", s.nodes, s.seconds, s.threads)
        elif s.proof != "cap":
            unknown(f"the search it repeats (board {source}) has proof {s.proof}, which has no search to price")
        elif source not in gaps:
            unknown(f"the search it repeats (board {source}) was proven at its ceiling, and --gaps has no gap for it")
        else:
            priced("gap model", gap_model_nodes(n, gaps[source]), gap=gaps[source])
    return rec


def group_games(games):
    """(members, cost) of the games that need a certificate, in increasing n: members maps each head, a game that has
    no share, to the boards of its group (the head, and the games that share with it directly or through a chain of
    shares), increasing; cost maps every such board to its expected thread-hours (0 where unknown)."""
    needing = [g for g in games if g["needs"] == 1]
    cost = {g["n"]: g["expected_thread_hours"] or 0.0 for g in needing}
    members = {}
    head_of = {}
    for g in needing:
        head = g["n"] if g["share"] is None else head_of[g["share"]]
        head_of[g["n"]] = head
        members.setdefault(head, []).append(g["n"])
    return members, cost


def dearest_heads(members, cost):
    """The heads of the groups, the dearest head first (ties to the lower n; the games that share cost nothing)."""
    return sorted(members, key=lambda head: (-cost[head], head))


def spread(heads, members, cost, k, dearest_first=False):
    """The groups of `heads` in k lists, balanced by expected hours: the heads in increasing n (the dearest first, if
    `dearest_first`: closer totals), each to the list with the smallest total so far (ties to the lowest list).
    Returns the k lists of heads."""
    totals = [0.0] * k
    lists = [[] for _ in range(k)]
    for head in sorted(heads, key=(lambda h: (-cost[h], h)) if dearest_first else None):
        i = min(range(k), key=lambda j: (totals[j], j))
        lists[i].append(head)
        totals[i] += sum(cost[n] for n in members[head])
    return lists


def make_lists(games, k):
    """The lists of the games that need a certificate: pilot, dear and k balanced rest lists, as dicts."""
    needing = [g for g in games if g["needs"] == 1]  # in increasing n
    members, cost = group_games(games)
    dearest = dearest_heads(members, cost)
    pilot, dear = dearest[:PILOT_HEADS], dearest[PILOT_HEADS:DEAR_HEADS]
    rest = spread([h for h in members if h not in set(dearest[:DEAR_HEADS])], members, cost, k)
    described = [
        ("pilot", f"the groups of the {PILOT_HEADS} dearest heads", pilot),
        ("dear", f"the groups of the next {DEAR_HEADS - PILOT_HEADS} dearest heads", dear),
    ] + [(f"rest_{i + 1}", f"list {i + 1} of {k} balanced lists of the other groups", h) for i, h in enumerate(rest)]
    lists = []
    for name, what, heads in described:
        boards = sorted(n for head in heads for n in members[head])
        hours = sum(cost[n] for n in boards)
        lists.append({"name": name, "what": what, "groups": len(heads), "games": boards, "hours": hours})
    if sorted(n for entry in lists for n in entry["games"]) != [g["n"] for g in needing]:
        raise AssertionError("a game that needs a certificate is in no list or in two")
    return lists


def build_plan(args, command):
    """The Plan of the arguments: reads the inputs, checks them, prices every board of the range."""
    if args.machine_threads < 1 or args.overhead <= 0 or args.lists < 1:
        raise PlanError("--machine-threads, --overhead and --lists must be positive")
    results = load_results(args.results)
    if not results:
        raise PlanError("the results have no completed line")
    upper = load_upper(args.upper)
    problems = load_problems(args.problems)
    gaps = load_gaps(args.gaps)
    lo = args.start
    hi = max(results) if args.end is None else args.end
    if lo > hi:
        raise PlanError(f"the range {lo}..{hi} is empty")
    eras = calibrate([parse_era(text) for text in args.era], results, args.machine_threads)
    for n in range(lo, hi + 1):
        if not any(era.lo <= n <= era.hi for era in eras):
            raise PlanError(f"board {n} is in the range {lo}..{hi}, but no --era holds it")
    check_inputs(lo, hi, upper, results, problems)
    games = [game_record(n, lo, hi, upper, results, problems, gaps, eras, args.overhead) for n in range(lo, hi + 1)]
    files = {"results": args.results, "upper": args.upper, "problems": args.problems, "gaps": args.gaps}
    return Plan(command, files, lo, hi, eras, args.machine_threads, args.overhead, args.price_on_demand,
                args.price_spot, games, make_lists(games, args.lists))


# --- the report ------------------------------------------------------------------------------------------------------


def hours_text(x):
    return "-" if x is None else f"{x:,.2f}"


def count_text(x):
    return "-" if x is None else f"{round(x):,}"


def table(headers, rows, align):
    """A markdown table; align has one letter a column: l or r."""
    lines = ["| " + " | ".join(headers) + " |", "|" + "|".join("---:" if a == "r" else "---" for a in align) + "|"]
    return lines + ["| " + " | ".join(str(cell) for cell in row) + " |" for row in rows]


def sum_hours(games, key="expected_thread_hours"):
    return sum(g[key] or 0.0 for g in games)


def tally(games):
    """The cells of the band table for some games of the plan."""
    needing = [g for g in games if g["needs"] == 1]
    return [
        f"{len(games):,}",
        len(needing),
        sum(g["proof"] == "same_search" for g in needing),
        sum(g["share"] is not None for g in needing),
        sum(g["proof"] == "exhausted" for g in needing),
        sum(g["relation"] == "prime" for g in games),
        sum(g["relation"] == "ceiling" for g in games),
        hours_text(sum_hours(games)),
    ]


def dearest_games(games):
    """The games of the greatest expected cost, dearest first (ties to the lower n)."""
    costly = [g for g in games if (g["expected_thread_hours"] or 0.0) > 0]
    return sorted(costly, key=lambda g: (-g["expected_thread_hours"], g["n"]))[:DEAREST_SHOWN]


def cost_origin(g):
    """Where the cost of a game comes from, for the table of the dearest."""
    named = {"chain": "itself", "source": f"board {g['source']}", "gap model": f"gap of {g['source']}"}
    return named.get(g["basis"], "-")


def section_inputs(plan):
    """The command, the files, the eras with their c, the overhead and the prices."""
    T = plan.machine_threads
    out = ["# Certificates campaign plan", ""]
    out += [f"- command: `{plan.command}`", f"- range: {plan.lo}..{plan.hi} ({len(plan.games):,} boards)"]
    out += [f"- {flag}: " + (", ".join(paths) if paths else "none") for flag, paths in plan.files.items()]
    if not plan.files["problems"]:
        out.append("- without problems no share is known (a repeat is costed in full) and |R| is null")
    out += [
        f"- machine: {T} threads",
        f"- overhead: a certificate costs {plan.overhead:g} times the plain search of its problem",
        f"- prices: ${plan.price_on_demand:g} an hour on demand, ${plan.price_spot:g} an hour at spot",
        "",
        "## The eras: what a node costs",
        "",
        "The chain ran several searches at once (a window), so a game's recorded search seconds are the time it",
        "shared the machine, not what it cost.  The cost is taken from nodes: c is the era's wall hours x 3600 x the",
        "machine's threads / the nodes of all its completed games.  Recorded / wall is the window's inflation.",
        "",
    ]
    rows = [
        [f"{e.lo}-{e.hi}", f"{e.games:,}", f"{e.hours:g}", f"{e.hours * T:,.1f}", f"{e.recorded:,.1f}",
         f"{e.recorded / (e.hours * T):.2f}", f"{e.nodes:,}", f"{e.c * 1e6:,.2f}"]
        for e in plan.eras
    ]
    out += table(["era", "games", "wall hours", "wall thread-hours", "recorded thread-hours", "recorded / wall",
                  "nodes", "c (us per node)"], rows, "lrrrrrrr")
    model = f"10^({GAP_INTERCEPT:g} + {GAP_SLOPE:g} G / n^{GAP_POWER:g} + {WINDOW_DECADES:g})"
    out += ["", "A repeat whose source was proven at its ceiling has no finished search to price; its nodes are the",
            f"cost model's {model} for the source's root gap G.", ""]
    return out


def section_bands(plan):
    """The table by band of 100 boards and the totals of the two parts and of all."""
    games = plan.games
    heads = ["boards", "games", "need a certificate", "repeats", "shared", "searched", "prime", "ceiling",
             "expected thread-hours"]
    rows = []
    for start in range(plan.lo - plan.lo % BAND, plan.hi + 1, BAND):
        band = [g for g in games if start <= g["n"] < start + BAND]
        rows.append([f"{max(plan.lo, start)}-{min(plan.hi, start + BAND - 1)}"] + tally(band))
    first, second = [g for g in games if g["n"] <= PART_END], [g for g in games if g["n"] > PART_END]
    for part in [first, second] if first and second else []:  # one part alone would repeat the total of all
        rows.append([f"**{part[0]['n']}-{part[-1]['n']}**"] + [f"**{cell}**" for cell in tally(part)])
    rows.append([f"**all {plan.lo}-{plan.hi}**"] + [f"**{cell}**" for cell in tally(games)])
    return ["## By band of 100 boards", ""] + table(heads, rows, "l" + "r" * 8) + [
        "",
        "Repeats, shared and searched count games that need a certificate; a shared game is usually a repeat too.",
        "",
    ]


def section_dearest(plan):
    """The table of the dearest games."""
    T = plan.machine_threads
    rows = [
        [g["n"], g["proof"], g["basis"], cost_origin(g), g["open"], count_text(g["rejected"]),
         count_text(g["cost_nodes"]), "-" if g["cost_seconds"] is None else f"{g['cost_seconds']:,.1f}",
         count_text(g["cost_threads"]), hours_text(g["expected_thread_hours"]),
         hours_text(g["expected_thread_hours"] / T)]
        for g in dearest_games(plan.games)
    ]
    heads = ["n", "proof", "basis", "cost from", "open", "rejected", "nodes", "chain seconds", "chain threads",
             "thread-hours", f"hours on {T} threads"]
    return [f"## The {DEAREST_SHOWN} dearest games", ""] + table(heads, rows, "llllrrrrrrr") + [""]


def section_named(plan):
    """What the plan says of the boards 3424, 3425 and 3440."""
    out = ["## Boards named by the plan", ""]
    named = [n for n in SPOTLIGHT if plan.lo <= n <= plan.hi]
    for n in named:
        g = plan.games[n - plan.lo]
        if g["needs"] == 1:
            out.append(f"- {n}: needs a certificate (its optimality rests on the frame model until it is certified); "
                       f"expected {hours_text(g['expected_thread_hours'])} thread-hours ({g['basis']})")
        else:
            out.append(f"- {n}: needs no certificate (relation {g['relation']}); expected 0 thread-hours")
    if not named:
        out.append("- none of " + ", ".join(map(str, SPOTLIGHT)) + f" is in the range {plan.lo}..{plan.hi}")
    return out + [""]


def section_estimate(plan):
    """The total, the machine-hours and dollars, the floor on wall time, the lists' totals and the recorded total."""
    T = plan.machine_threads
    needing = [g for g in plan.games if g["needs"] == 1]
    unknown = [g for g in needing if g["basis"] == "unknown"]
    total = sum_hours(plan.games)
    recorded = sum_hours(plan.games, "expected_thread_hours_recorded")
    machine_hours = total / T
    out = [
        "## The estimate",
        "",
        f"- total: {total:,.1f} thread-hours over {len(needing):,} games that need a certificate"
        + (f" ({len(unknown)} of unknown cost are not in it)" if unknown else ""),
        f"- on one machine of {T} threads: {machine_hours:,.1f} hours ({machine_hours / 24:,.1f} days)",
        f"- on demand at ${plan.price_on_demand:g} an hour: ${machine_hours * plan.price_on_demand:,.2f}; "
        f"at spot at ${plan.price_spot:g} an hour: ${machine_hours * plan.price_spot:,.2f}",
    ]
    dearest = dearest_games(plan.games)
    if dearest:
        g = dearest[0]
        out.append(f"- floor on wall time: {g['expected_thread_hours'] / T:,.2f} hours on one machine, the dearest "
                   f"game ({g['n']}, {g['expected_thread_hours']:,.1f} thread-hours) alone")
    listed = ", ".join(f"{entry['name']} {entry['hours']:,.1f}" for entry in plan.lists)
    together = sum_hours(plan.lists, "hours")
    out.append(f"- the lists' expected totals (thread-hours): {listed} (together {together:,.1f})")
    out.append(f"- the same total from the recorded seconds instead of nodes: {recorded:,.1f} thread-hours, "
               f"{recorded / T:,.1f} hours on one machine; the upper end, which ignores the window")
    return out + [""]


def section_exceptions(plan):
    """The games of unknown cost and the games whose share is refused."""
    needing = [g for g in plan.games if g["needs"] == 1]
    out = ["## Games of unknown cost", ""]
    out += [f"- {g['n']}: {g['reason']}" for g in needing if g["basis"] == "unknown"] or ["none"]
    out += ["", "## Games whose share is refused (same_as is set, but the worths differ)", ""]
    out += [f"- {g['n']} (same_as {g['same_as']})" for g in needing if g["share_refused"]] or ["none"]
    return out


def render(plan):
    """The text of plan.md."""
    sections = [section_inputs, section_bands, section_dearest, section_named, section_estimate, section_exceptions]
    return "\n".join(line for section in sections for line in section(plan)) + "\n"


def list_text(entry, command):
    """The text of a list file."""
    head = [
        f"# {entry['name']}: {entry['what']}",
        f"# games: {len(entry['games'])} in {entry['groups']} groups",
        f"# expected thread-hours: {entry['hours']:,.2f}",
        f"# command: {command}",
    ]
    return "\n".join(head + [str(n) for n in entry["games"]]) + "\n"


def write_outputs(plan, text, out_dir):
    """plan.jsonl, plan.md and lists/*.txt in out_dir (the lists of an earlier run are removed first)."""
    lists_dir = out_dir / "lists"
    lists_dir.mkdir(parents=True, exist_ok=True)
    for stale in [lists_dir / "pilot.txt", lists_dir / "dear.txt", *lists_dir.glob("rest_*.txt")]:
        stale.unlink(missing_ok=True)
    (out_dir / "plan.jsonl").write_text("".join(json.dumps(g) + "\n" for g in plan.games))
    (out_dir / "plan.md").write_text(text)
    for entry in plan.lists:
        (lists_dir / f"{entry['name']}.txt").write_text(list_text(entry, plan.command))


def parse_args(argv):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--results", nargs="+", action="extend", required=True, metavar="FILE")
    ap.add_argument("--upper", nargs="+", action="extend", required=True, metavar="FILE")
    ap.add_argument("--problems", nargs="+", action="extend", default=[], metavar="FILE")
    ap.add_argument("--gaps", nargs="+", action="extend", default=[], metavar="FILE")
    ap.add_argument("--from", dest="start", type=int, default=FIRST_BOARD, metavar="A")
    ap.add_argument("--to", dest="end", type=int, default=None, metavar="B")
    ap.add_argument("--era", action="append", required=True, metavar="A-B:HOURS")
    ap.add_argument("--machine-threads", type=int, default=MACHINE_THREADS)
    ap.add_argument("--overhead", type=float, default=OVERHEAD)
    ap.add_argument("--price-on-demand", type=float, default=PRICE_ON_DEMAND)
    ap.add_argument("--price-spot", type=float, default=PRICE_SPOT)
    ap.add_argument("--lists", type=int, default=LISTS)
    ap.add_argument("--out-dir", type=Path, default=None)
    return ap.parse_args(argv)


def main(argv=None):
    argv = sys.argv[1:] if argv is None else list(argv)
    args = parse_args(argv)
    command = shlex.join(["certify_plan.py", *argv])
    try:
        plan = build_plan(args, command)
        text = render(plan)
        if args.out_dir is not None:
            write_outputs(plan, text, args.out_dir)
    except PlanError as e:
        print(f"certify_plan.py: error: {e}", file=sys.stderr)
        return 1
    except OSError as e:
        print(f"certify_plan.py: error: cannot write the output: {e}", file=sys.stderr)
        return 1
    sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
