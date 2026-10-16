#!/usr/bin/env python3
"""Where the search workers' time goes, from a macOS `sample` profile of `taxman solve` (run.sh with a profile).

usage: groups.py BINARY sample.txt [--top K] [--json]     (from rust/bench/frontier)

Only the engine worker threads (`engine-N`) are counted: the main thread and the game's setup thread wait or set the
game up.  Every frame's return address is symbolicated with its inlined frames (`atos -i`, as tools/profile/attribute.py
does, whose parser this uses), so a function the compiler inlined is charged as itself.  Three tables:

* inclusive and self time by function: a sample counts once for each function on its stack (inclusive), and for the
  innermost function of its leaf (self; `sample` symbolicates only the first address of a coalesced leaf line, so
  below a function the attribution is coarse);
* the groups, a partition of the busy samples: the first rule that a sample's stack satisfies (GROUPS, in order);
* the groups by the kernel that ran (the leaf's function), for the GF(2) bound's own breakdown.

Shares are of the busy samples (the workers' samples outside waits), with the idle share of all of them apart.
"""
import collections
import json
import re
import sys
from pathlib import Path

# The profile tool's parser and symbolicator (tools/profile/attribute.py), one source for both.
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "tools" / "profile"))
import attribute  # noqa: E402

WORKER = re.compile(r"engine-\d+")
WAITS = ("psynch", "__ulock", "semwait", "semaphore_wait", "kevent", "nanosleep", "cond_wait", "cond_timedwait")
ALLOC = ("_xzm", "malloc", "free", "calloc", "realloc", "nanov2", "szone", "alloc::alloc", "alloc::raw_vec")


def has(names, *parts):
    """Whether some function of the stack has one of `parts` in its name."""
    return any(part in name for name in names for part in parts)


def ends(names, *tails):
    return any(name.endswith(tail) for name in names for tail in tails)


# (group, test of the stack's functions root first and the leaf's innermost function), the first to hold wins.
def in_step(s):
    return has(s, "TaxSearcher::step")


GROUPS = [
    ("checkpoints and pauses", lambda s, leaf: has(s, "write_checkpoint", "begin_pause", "end_pause", "spill")),
    ("allocation (malloc, free, Vec growth)", lambda s, leaf: any(a in leaf for a in ALLOC)),
    ("work units and the deque (engine, poll, rebuild of a unit)",
     lambda s, leaf: (has(s, "taxman:engine::") and not in_step(s)) or ends(s, "TaxSearcher::start")),
    ("strong branching (choose_branch_gf2: exact bounds without each member)",
     lambda s, leaf: ends(s, "Stepper::choose_branch_gf2")),
    ("forced members (shortest cycles of the exchange graph)", lambda s, leaf: has(s, "force_by_cycles")),
    ("GF(2) bound: weight split (split_bounds)", lambda s, leaf: has(s, "Meet::split_bounds")),
    ("GF(2) bound: exchange graph rebuilt after a solve (Meet::rebuild)", lambda s, leaf: has(s, "Meet::rebuild")),
    ("GF(2) bound: augmenting paths (Meet::solve, shortest_path, apply_path)",
     lambda s, leaf: has(s, "Meet::shortest_path", "Meet::apply_path", "Meet::solve", "Meet::exchange")),
    ("GF(2) bound: the rest (evaluate, levels pushed and popped, certify_child)",
     lambda s, leaf: has(s, "gf2::")),
    ("nogood table and conflicts (lookups, rule-outs, learning)",
     lambda s, leaf: has(s, "nogoods::", "FrameGraph::nogood", "Seek::conflict", "model::conflict")),
    ("frame checks (promotion, winnable, peel)", lambda s, leaf: has(s, "peel::", "model::promote", "max_additional")),
    ("symmetry rule", lambda s, leaf: has(s, "symmetry::")),
    ("the step's own work (loop, offers, bookkeeping)", lambda s, leaf: True),
]

# The kernel of a sample by its leaf's function (first match).
KERNELS = [
    ("Meet::shortest_cycle (cycles: forced members, bounds without a member)", "Meet::shortest_cycle"),
    ("Meet::add (the exchange graph's edges)", "Meet::add"),
    ("Meet::split_bounds (the weight split's bounds)", "Meet::split_bounds"),
    ("Meet::shortest_path (augmenting path)", "Meet::shortest_path"),
    ("Meet::smaller_loss", "Meet::smaller_loss"),
    ("Meet::build_columns", "Meet::build_columns"),
    ("Meet other (rebuild, resolve, xor, BlockSpan, is_zero, ...)", ("Meet::", "meet::")),
    ("gf2 other (evaluate, contract, certify, Gf2Graph)", "gf2::"),
    ("peel (winnable, Stack::push)", "peel::"),
    ("conflicts and nogoods", ("conflict", "nogoods", "refine::Seek")),
    ("symmetry", "symmetry::"),
    ("promote and narrow", "promote::"),
    ("memcpy, memset", ("memmove", "memcpy", "memset")),
    ("engine", "taxman:engine"),
    ("the step (searcher.rs)", "searcher::"),
    ("bits (Bits, iterators)", "bits::"),
]


def kernel(leaf):
    for label, parts in KERNELS:
        parts = (parts,) if isinstance(parts, str) else parts
        if any(p in leaf for p in parts):
            return label
    return "other: " + leaf


def frame_name(frame):
    """A frame outside the solver: its symbol, less the offsets."""
    return frame.split("  (in")[0].strip()


def functions(chain, frame):
    """The functions of one frame, outermost first: the solver's functions of its inline chain (the helpers of the
    standard library inlined into them are dropped), or the frame's own symbol where the chain has none."""
    ours = [f for f in chain if f.startswith("taxman:")]
    return list(reversed(ours)) if ours else [frame_name(frame)]


def analyse(binary, path, top):
    load, leaves = attribute.parse(path)
    leaf_addresses = {attribute.address(stack[-1]) for _, stack in leaves} - {None}
    # A caller's address is a return address: the call is the byte before it, which the inline chain is about.
    call_addresses = {hex(int(a, 16) - 1) for _, stack in leaves for a in map(attribute.address, stack[:-1]) if a}
    leaf_chains = attribute.symbolicate(binary, load, leaf_addresses)
    call_chains = attribute.symbolicate(binary, load, call_addresses)
    inclusive, innermost = collections.Counter(), collections.Counter()
    groups, kernels = collections.Counter(), collections.Counter()
    by_group_kernel = collections.Counter()
    idle = busy = 0
    for count, stack in leaves:
        if not WORKER.search(stack[0]):
            continue
        names = []
        for frame in stack[1:-1]:
            address = attribute.address(frame)
            chain = call_chains.get(hex(int(address, 16) - 1), []) if address else []
            names.extend(functions(chain, frame))
        names.extend(functions(leaf_chains.get(attribute.address(stack[-1]), []), stack[-1]))
        leaf = names[-1]
        if any(w in stack[-1] for w in WAITS):
            idle += count
            continue
        busy += count
        for name in set(names):
            inclusive[name] += count
        innermost[leaf] += count
        group = next(g for g, test in GROUPS if test(names, leaf))
        groups[group] += count
        kernels[kernel(leaf)] += count
        by_group_kernel[(group, kernel(leaf))] += count
    share = lambda c: 100.0 * c / busy  # noqa: E731
    return {
        "file": f"{Path(path).parent.name}/{Path(path).name}",
        "busy_samples": busy,
        "idle_samples": idle,
        "idle_share": 100.0 * idle / max(1, idle + busy),
        "groups": [(g, round(share(c), 2)) for g, c in groups.most_common()],
        "kernels": [(k, round(share(c), 2)) for k, c in kernels.most_common()],
        "group_kernel": [(g, k, round(share(c), 2)) for (g, k), c in by_group_kernel.most_common(top)],
        "inclusive": [(n, round(share(c), 2)) for n, c in inclusive.most_common(top)],
        "self": [(n, round(share(c), 2)) for n, c in innermost.most_common(top)],
    }


def show(result):
    print(f"== {result['file']}: {result['busy_samples']} busy worker samples, idle {result['idle_share']:.1f}%")
    for title, key in (("groups", "groups"), ("kernels (leaf)", "kernels")):
        print(f" {title}:")
        for label, pct in result[key]:
            print(f"  {pct:5.1f}%  {label}")
    print(" group x kernel:")
    for g, k, pct in result["group_kernel"]:
        print(f"  {pct:5.1f}%  {g} | {k}")
    for title, key in (("inclusive", "inclusive"), ("self (innermost function of the leaf)", "self")):
        print(f" {title}:")
        for name, pct in result[key]:
            print(f"  {pct:5.1f}%  {name}")


def main():
    args = sys.argv[1:]
    top = int(args[args.index("--top") + 1]) if "--top" in args else 30
    result = analyse(args[0], args[1], top)
    if "--json" in args:
        print(json.dumps(result))
    else:
        show(result)


main()
