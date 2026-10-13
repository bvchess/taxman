"""Attribute macOS `sample` profiles of the solver to kinds of work and their purposes (docs/IDEAS.md, section 2).

usage: attribute.py BINARY sample.txt [sample.txt ...]   (run by profile.sh, with the binary that was sampled)

Each frame's return address is symbolicated with its inlined frames (`atos -i`), so that code the compiler inlined
into the step is charged to the function it came from.  The kind of work is the innermost function of the crate in
the leaf's inline chain (matching repairs, bounds from the potentials, peels, conflicts, nogood table, the node's
own work, copies); the purpose is the innermost search function (strong branching, refinement, the child's check,
...) found walking up from the leaf through the inline chains.  Standard library only; macOS only (`atos`).
"""
import collections
import re
import subprocess
import sys

binary = sys.argv[1]
LINE = re.compile(r'^([ +!:|]*)(\d+) (.*)$')


def parse(path):
    """(load address, [(self count, stack)]) with stack root first, each frame its text."""
    text = open(path).read()
    load = re.search(r'Load Address:\s+(0x[0-9a-f]+)', text).group(1)
    lines = text.split('\n')
    start = lines.index('Call graph:') + 1
    stack = []
    out = []

    def pop_to(depth):
        while stack and stack[-1][0] >= depth:
            d, c, t, kids = stack.pop()
            if c - kids > 0:
                out.append((c - kids, [s[2] for s in stack] + [t]))

    for raw in lines[start:]:
        if not raw.strip():
            break
        m = LINE.match(raw)
        if not m:
            continue
        depth, count, t = len(m.group(1)), int(m.group(2)), m.group(3)
        pop_to(depth)
        if stack:
            d, c, tt, kids = stack[-1]
            stack[-1] = (d, c, tt, kids + count)
        stack.append((depth, count, t, 0))
    pop_to(-1)
    return load, out


def address(frame):
    m = re.search(r'\[(0x[0-9a-f]+)', frame)
    return m.group(1) if m and '(in taxman)' in frame else None


def symbolicate(load, addrs):
    chains = {}
    addrs = sorted(addrs)
    for i in range(0, len(addrs), 400):
        batch = addrs[i:i + 400]
        out = subprocess.run(['atos', '-i', '-o', binary, '-l', load] + batch, capture_output=True, text=True).stdout
        blocks = out.strip('\n').split('\n\n')
        for a, block in zip(batch, blocks):
            chains[a] = [short(line) for line in block.split('\n') if line.strip()]
    return chains


def idents(sym):
    """The identifiers of a v0-mangled Rust symbol, in order (<decimal length><identifier> tokens)."""
    out, i = [], 0
    while i < len(sym):
        if sym[i].isdigit() and (i == 0 or not sym[i - 1].isdigit()):
            j = i
            while j < len(sym) and sym[j].isdigit():
                j += 1
            n = int(sym[i:j])
            if j < len(sym) and sym[j] == '_':
                # A base-62 number (a back reference, a disambiguator, a constant), not an identifier's length.
                i = j + 1
                continue
            word = sym[j:j + n]
            if n > 0 and len(word) == n and re.fullmatch(r'[A-Za-z_][A-Za-z0-9_]*', word):
                out.append(word)
                i = j + n
                continue
        i += 1
    return out


def short(line):
    """'taxman:module::Type::function' for the crate's functions, 'std:...' for others."""
    sym = line.split(' (in ')[0]
    marks = {m: sym.find(m) for m in ('4core', '3std', '5alloc', '6taxman')}
    found = {m: i for m, i in marks.items() if i >= 0}
    if found and min(found, key=found.get) == '6taxman':
        words = [w for w in idents(sym[found['6taxman']:]) if len(w) > 1]
        rest = []
        for w in words[1:]:
            if w in ('core', 'std', 'alloc', 'taxman'):
                break
            rest.append(w)
        return 'taxman:' + '::'.join(rest[:4])
    return 'std:' + '::'.join([w for w in idents(sym) if len(w) > 1][-2:])


PURPOSES = [
    ('strongest', 'strong branching (exact bounds without each member)'),
    ('branches', 'refinement (bounds of the conflict\'s branches)'),
    ('reaches', 'refinement (bounds of the conflict\'s branches)'),
    ('find_conflict', 'conflict lookup or computation'),
    ('top', 'node check: forced members, conflict'),
    ('check_child', "child's check (matching bound of the child)"),
    ('make_child', 'making the child (promotion, candidates, rule-outs)'),
    ('choose_branch', 'choosing the branch'),
    ('check_node', 'node check'),
    ('start', 'rebuilding a unit'),
    ('check', 'node or child check'),
    ('offer', 'offering an incumbent'),
]


def purpose(chains_up):
    for chain in chains_up:
        for f in chain:
            name = f.split('::')[-1]
            for key, label in PURPOSES:
                if name == key:
                    return label
    return '-'


def kind(chain, frame):
    if any(w in frame for w in ('psynch', '__ulock', 'semwait', 'kevent', 'nanosleep')):
        return 'idle (waiting)'
    if 'memmove' in frame or 'memcpy' in frame or 'memset' in frame:
        return 'copies (memmove/memset)'
    ours = [f for f in chain if f.startswith('taxman:')]
    if not ours:
        return 'other'
    name = ours[0]
    if 'bound_excluding' in name or 'bound_including' in name or 'fixed_out' in name:
        return 'bounds from the potentials'
    if 'copy_from' in name or 'push_copy' in name:
        return 'matching level copies'
    if name.startswith('taxman:matchbound'):
        return 'matching repairs (Hungarian)'
    if name.startswith('taxman:model::nogoods'):
        return 'nogood table'
    if name.startswith('taxman:model::conflict') or any(f.startswith('taxman:model::conflict') for f in ours):
        return 'conflict minimization (peels of the core)'
    if name.startswith('taxman:model'):
        return 'peels and promotions'
    if name.startswith('taxman:engine'):
        return 'engine'
    return "node's own work"


for path in sys.argv[2:]:
    load, nodes = parse(path)
    addrs = set()
    for _, stack in nodes:
        for f in stack:
            a = address(f)
            if a:
                addrs.add(a)
    chains = symbolicate(load, addrs)
    by_kind = collections.Counter()
    by_purpose = collections.Counter()
    by_both = collections.Counter()
    by_leaf = collections.Counter()
    for count, stack in nodes:
        leaf = stack[-1]
        leaf_chain = chains.get(address(leaf), [])
        k = kind(leaf_chain, leaf)
        by_kind[k] += count
        if k == 'idle (waiting)':
            continue
        up = [leaf_chain] + [chains.get(address(f), []) for f in reversed(stack[:-1])]
        p = purpose(up)
        by_purpose[p] += count
        by_both[(k, p)] += count
        by_leaf[next((f for f in leaf_chain if f.startswith("taxman:")), None) or leaf.split('  (in')[0][:60]] += count
    busy = sum(c for k, c in by_kind.items() if k != 'idle (waiting)')
    pct = lambda c: f'{100 * c / busy:5.1f}%'
    print(f'== {path}: {busy} busy samples, {by_kind["idle (waiting)"]} idle')
    print(' kind of work:')
    for k, c in by_kind.most_common():
        if k != 'idle (waiting)':
            print(f'  {pct(c)}  {k}')
    print(' purpose:')
    for p, c in by_purpose.most_common():
        print(f'  {pct(c)}  {p}')
    print(' kind x purpose (>= 1%):')
    for (k, p), c in by_both.most_common():
        if c / busy >= 0.01:
            print(f'  {pct(c)}  {k:40} | {p}')
    print(' innermost functions (self, top 15):')
    for n, c in by_leaf.most_common(15):
        print(f'  {pct(c)}  {n}')
