#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The list files of the stage 4 campaign (phases A, B and C), made from the work list, and their check.

Standard library only; every path comes from the arguments, so it runs from anywhere.

The work list (`solver/tools/certify/plan.jsonl`, made by certify_plan.py) says which games need a certificate and which
of them the rule of the same problem settles from another game's certificate (`share`).  A game with no share is a head;
the games that share with it, directly or through a chain of shares, are its group.  The rule reads only the
certificates of the `--out` file it appends to, so a group goes whole into one list, its head first.  The campaign runs
in phases (solver/docs/runbook/certify.md):

  pilot7     phase A.  The seven games of PILOT_HEADS with their groups, the dearest head first.  Run alone, one game
             after the other on all the threads, so that each game's time per node is measured cleanly.
  dear_rest  phase B, first.  The dear groups the pilot leaves: the heads among the DEAR_HEADS dearest of the plan
             (certify_plan.py's pilot and dear lists) that are neither in the pilot nor deferred, the dearest head
             first, each group whole (3440, 3424, 3425 and 2346 lead).  All the threads, one game after the other.
  rest_1..4  phase B, then.  Every other group but the deferred ones, balanced by expected hours (the dearest group
             to the lightest list so far, certify_plan.spread), each list in increasing n: four processes of a
             quarter of the threads each.
  deferred   phase C.  The repeats of a search that the rule cannot settle (proof same_search, no share) with their
             groups, in increasing n, last: a rule that knows unzip's claims in the extended problem (the kept-claims
             lemma, E4, docs/theory/extended-problem.md) may settle them with no search once it is built, from the
             certificates of the earlier phases.

No game is in two lists, every game that needs a certificate is in one, and a game that shares has the game it shares
with earlier in the same list (`check` verifies all three of any directory of lists).

usage: certify_campaign.py lists --plan PLAN.jsonl --out-dir DIR [--rest K]
       certify_campaign.py check --plan PLAN.jsonl --lists-dir DIR
The exit status is 1 if an input is wrong, or the lists fail the check (each problem is printed).
"""
import argparse
import json
import re
import shlex
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))  # python -I leaves the script's own directory off the path
import certify_plan  # noqa: E402

PILOT_HEADS = (2340, 3175, 3300, 3335, 3336, 3408, 3420)  # |O| 192-271, the 2300s and the 3100s-3400s, 3175's 4 words
REST_LISTS = 4  # four processes of 16 threads on the 64 of a c8g.16xlarge
DEFERRED = "deferred"
PLAN_FIELDS = ("n", "needs", "share", "proof", "expected_thread_hours")


class CampaignError(Exception):
    """An input is wrong; the message says what."""


def read_text(path):
    try:
        return Path(path).read_text()
    except (OSError, UnicodeDecodeError) as e:
        raise CampaignError(f"{path}: cannot read: {getattr(e, 'strerror', None) or e}") from e


def read_plan(path):
    """The fields of plan.jsonl that the lists use, one dict a board, in order."""
    games = []
    for number, line in enumerate(read_text(path).splitlines(), 1):
        if not line.strip():
            continue
        try:
            game = json.loads(line)
            games.append({key: game[key] for key in PLAN_FIELDS})
        except (json.JSONDecodeError, KeyError, TypeError) as e:
            raise CampaignError(f"{path} line {number}: not a line of the work list: {e!r}") from e
    return games


def unsettled_repeats(games):
    """The games that repeat a search and have no share: the rule of the same problem cannot settle them."""
    return {g["n"] for g in games if g["needs"] == 1 and g["proof"] == "same_search" and g["share"] is None}


def campaign_lists(games, rest=REST_LISTS):
    """The campaign's lists as dicts (name, phase, what, groups, games, hours), in the order they run."""
    members, cost = certify_plan.group_games(games)
    deferred_heads = sorted(unsettled_repeats(games))
    for head in PILOT_HEADS:
        if head not in members or head in deferred_heads:
            raise CampaignError(f"the pilot game {head} is not a game that the rule leaves to a search")
    dearest = certify_plan.dearest_heads(members, cost)
    dear_heads = [h for h in dearest[: certify_plan.DEAR_HEADS] if h not in PILOT_HEADS and h not in deferred_heads]
    taken = set(PILOT_HEADS) | set(dear_heads) | set(deferred_heads)
    rest_heads = certify_plan.spread([h for h in members if h not in taken], members, cost, rest, dearest_first=True)

    def by_cost(heads):
        """The boards of the groups of `heads`, the dearest head first, and how many groups."""
        ordered = sorted(heads, key=lambda head: (-cost[head], head))
        return [n for head in ordered for n in members[head]], len(ordered)

    def increasing(heads):
        return sorted(n for head in heads for n in members[head]), len(heads)

    pilot_what = f"the {len(PILOT_HEADS)} games of the pilot with their groups, the dearest first"
    deferred_what = "the repeats the rule cannot settle with their groups, to be settled by a rule that knows unzip's "
    deferred_what += "claims in the extended problem if one is built, else searched"
    described = [("pilot7", "A", pilot_what, *by_cost(PILOT_HEADS)),
                 ("dear_rest", "B", "the other dear groups, the dearest first", *by_cost(dear_heads))]
    described += [(f"rest_{i}", "B", f"list {i} of {rest} balanced lists of the other groups", *increasing(heads))
                  for i, heads in enumerate(rest_heads, 1)]
    described.append((DEFERRED, "C", deferred_what, *increasing(deferred_heads)))
    return [{"name": name, "phase": phase, "what": what, "groups": groups, "games": boards,
             "hours": sum(cost[n] for n in boards)} for name, phase, what, boards, groups in described]


def list_text(entry, command):
    """The text of a list file: four header lines that start with `# `, then one n a line."""
    head = [
        f"# {entry['name']}: phase {entry['phase']}, {entry['what']}",
        f"# games: {len(entry['games'])} in {entry['groups']} groups",
        f"# expected thread-hours: {entry['hours']:,.2f}",
        f"# command: {command}",
    ]
    return "\n".join(head + [str(n) for n in entry["games"]]) + "\n"


def read_list(path):
    """The boards of a list file in its order, as `taxman certify --list` reads them: one n a line, blank lines and
    lines that start with `#` skipped.  A line that is not a game n, or a board listed twice, is an error."""
    boards = []
    first = {}
    for number, line in enumerate(read_text(path).splitlines(), 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if not re.fullmatch(r"[0-9]+", line):
            raise CampaignError(f"{path} line {number}: expected a game n, got {line!r}")
        n = int(line)
        if n in first:
            raise CampaignError(f"{path} line {number}: the board {n} is listed again (first on line {first[n]})")
        first[n] = number
        boards.append(n)
    return boards


def check_lists(games, lists):
    """The problems of `lists`, a dict name -> boards in order, against the work list: [] if there are none.

    Every game that needs a certificate is in one list and no other board is in any; no board is in two lists; a game
    that shares has the game it shares with earlier in its own list; `deferred` holds exactly the unsettled repeats and
    their groups, by increasing n."""
    problems = []
    need = {g["n"]: g for g in games if g["needs"] == 1}
    where = {}
    for name, boards in lists.items():
        for position, n in enumerate(boards):
            if n in where:
                problems.append(f"board {n} is in {where[n][0]} and in {name}")
            where[n] = (name, position)
            if n not in need:
                problems.append(f"board {n} of {name} does not need a certificate")
    problems += [f"board {n} needs a certificate and is in no list" for n in need if n not in where]
    for n, g in need.items():
        share = g["share"]
        if share is None or n not in where or share not in where:
            continue
        if where[share][0] != where[n][0]:
            problems.append(f"board {n} shares with {share}, which is in {where[share][0]}, not in {where[n][0]}")
        elif where[share][1] > where[n][1]:
            problems.append(f"board {n} comes before {share}, whose certificate it rests on, in {where[n][0]}")
    members, _cost = certify_plan.group_games(games)
    deferred = sorted(n for head in unsettled_repeats(games) for n in members[head])
    if DEFERRED in lists and lists[DEFERRED] != deferred:
        problems.append(f"{DEFERRED} is not the unsettled repeats with their groups, by increasing n")
    return problems


def write_lists(lists, out_dir, command):
    out_dir.mkdir(parents=True, exist_ok=True)
    for entry in lists:
        (out_dir / f"{entry['name']}.txt").write_text(list_text(entry, command))


def directory_lists(directory):
    """{name: boards} of the `*.txt` list files of a directory."""
    paths = sorted(Path(directory).glob("*.txt"))
    if not paths:
        raise CampaignError(f"{directory}: no list file (*.txt)")
    return {path.stem: read_list(path) for path in paths}


def summary(games, lists):
    """A line a list: games, groups, expected thread-hours, and the hours the 64 threads of a machine need for it."""
    threads = certify_plan.MACHINE_THREADS
    on_machine = f"hours on {threads}"
    out = [f"{'list':<10} {'phase':<5} {'games':>6} {'groups':>6} {'thread-hours':>13} {on_machine:>12}"]
    for entry in lists:
        out.append(f"{entry['name']:<10} {entry['phase']:<5} {len(entry['games']):>6} {entry['groups']:>6} "
                   f"{entry['hours']:>13,.2f} {entry['hours'] / threads:>12.2f}")
    total = sum(entry["hours"] for entry in lists)
    need = sum(1 for g in games if g["needs"] == 1)
    out.append(f"{'all':<10} {'':<5} {sum(len(e['games']) for e in lists):>6} {sum(e['groups'] for e in lists):>6} "
               f"{total:>13,.2f} {total / threads:>12.2f}   ({need} games need a certificate)")
    return out


def parse_args(argv):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="command", required=True)
    make = sub.add_parser("lists", help="write the campaign's list files")
    make.add_argument("--plan", required=True, metavar="PLAN.jsonl")
    make.add_argument("--out-dir", required=True, type=Path)
    make.add_argument("--rest", type=int, default=REST_LISTS, metavar="K", help="the balanced rest lists")
    test = sub.add_parser("check", help="check the list files of a directory against the work list")
    test.add_argument("--plan", required=True, metavar="PLAN.jsonl")
    test.add_argument("--lists-dir", required=True, type=Path)
    return ap.parse_args(argv)


def main(argv=None):
    argv = sys.argv[1:] if argv is None else list(argv)
    args = parse_args(argv)
    try:
        games = read_plan(args.plan)
        if args.command == "lists":
            if args.rest < 1:
                raise CampaignError("--rest must be at least 1")
            lists = campaign_lists(games, args.rest)
            write_lists(lists, args.out_dir, shlex.join(["certify_campaign.py", *argv]))
            problems = check_lists(games, {entry["name"]: entry["games"] for entry in lists})
            print("\n".join(summary(games, lists)))
        else:
            found = directory_lists(args.lists_dir)
            problems = check_lists(games, found)
            print(f"{len(found)} lists, {sum(len(boards) for boards in found.values())} games: "
                  + ", ".join(f"{name} {len(boards)}" for name, boards in found.items()))
    except CampaignError as e:
        print(f"certify_campaign.py: error: {e}", file=sys.stderr)
        return 1
    except OSError as e:
        print(f"certify_campaign.py: error: cannot write the output: {e}", file=sys.stderr)
        return 1
    for problem in problems:
        print(f"certify_campaign.py: {problem}", file=sys.stderr)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
