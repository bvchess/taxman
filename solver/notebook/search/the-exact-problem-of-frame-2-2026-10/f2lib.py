#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Frame 2's exact problem at a node, read from the instance lines of `wall local --out`."""

import gzip
import json
import math


def read_instances(path):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as f:
        for line in f:
            line = line.strip()
            if line:
                d = json.loads(line)
                if d.get("kind") == "instance":
                    yield d


_spf = {}


def smallest_prime_factor_table(n):
    spf = list(range(n + 1))
    i = 2
    while i * i <= n:
        if spf[i] == i:
            for j in range(i * i, n + 1, i):
                if spf[j] == j:
                    spf[j] = i
        i += 1
    return spf


def primes_of(x, spf):
    out = []
    while x > 1:
        p = spf[x]
        e = 0
        while x % p == 0:
            x //= p
            e += 1
        out.append((p, e))
    return out


def max_factors(x, spf):
    return [x // p for p, _ in primes_of(x, spf)]


class Frame2:
    """Frame `h` of a node (h = 2 by default): its moves, its factors, the open candidates of homes h and h + 1.

    movers: the frame's moves (fixed) and the open candidates of home h + 1 (optional movers).
    factors: the frame's factors; those that are open candidates of home h may be promoted out (optional removals).
    adj[y]: the live factors of mover y (its maximal factors among the frame's factors).
    """

    def __init__(self, inst, h=2, spf=None):
        self.n = inst["n"]
        self.h = h
        if spf is None:
            spf = smallest_prime_factor_table(self.n)
        self.spf = spf
        frames = inst["frames"]
        moves, factors, freedoms = frames[h]
        self.moves = set(moves)
        self.factors = set(factors)
        self.freedoms = freedoms
        cands = inst["cands"]
        self.c_in = sorted(x for x, home in cands if home == h + 1)  # optional movers
        self.c_out = sorted(x for x, home in cands if home == h and x in self.factors)  # optional removals
        self.movers = sorted(self.moves) + self.c_in
        self.adj = {y: [f for f in max_factors(y, spf) if f in self.factors] for y in self.movers}
        self.users = {f: [] for f in self.factors}
        for y in self.movers:
            for f in self.adj[y]:
                self.users[f].append(y)

    def residual(self, movers, factors):
        """The movers of `movers` that the peel cannot drop against `factors` (empty: winnable)."""
        live = set(movers)
        count = {}
        for y in live:
            for f in self.adj[y]:
                if f in factors:
                    count[f] = count.get(f, 0) + 1
        stack = [y for y in live if any(count.get(f, 0) == 1 for f in self.adj[y] if f in factors)]
        while stack:
            y = stack.pop()
            if y not in live:
                continue
            if not any(count.get(f, 0) == 1 for f in self.adj[y] if f in factors):
                continue
            live.discard(y)
            for f in self.adj[y]:
                if f in factors:
                    count[f] -= 1
                    if count[f] == 1:
                        for z in self.users[f]:
                            if z in live:
                                stack.append(z)
        return live

    def kernel(self):
        """The movers that can lie in a self-covering set of some configuration: the residual with every optional mover
        in and every optional factor out (a self-covering set of any configuration stays one there)."""
        return self.residual(self.movers, self.factors - set(self.c_out))


def bipartite_graph(fr, movers):
    """The incidence graph of `movers` and their live factors, as adjacency sets keyed ('m', y) and ('f', f)."""
    g = {}
    for y in movers:
        g.setdefault(("m", y), set())
        for f in fr.adj[y]:
            g[("m", y)].add(("f", f))
            g.setdefault(("f", f), set()).add(("m", y))
    return g


def components(g):
    seen = set()
    comps = []
    for v in g:
        if v in seen:
            continue
        stack = [v]
        seen.add(v)
        comp = []
        while stack:
            u = stack.pop()
            comp.append(u)
            for w in g[u]:
                if w not in seen:
                    seen.add(w)
                    stack.append(w)
        comps.append(comp)
    return comps


def two_core(g):
    """The graph less its vertices of degree at most 1, repeatedly."""
    g = {v: set(ns) for v, ns in g.items()}
    stack = [v for v in g if len(g[v]) <= 1]
    while stack:
        v = stack.pop()
        if v not in g:
            continue
        if len(g[v]) > 1:
            continue
        for w in g[v]:
            g[w].discard(v)
            if len(g[w]) <= 1:
                stack.append(w)
        del g[v]
    return g


def fibers(fr, movers, spf):
    """Splits the incidence graph of `movers` by the large prime: with T = floor(sqrt(n / 8)), every number of the
    frame has at most one prime above T, to the first power.  Returns T, the graph, the core (T-smooth vertices) and
    the fibers {p: vertices with the prime p > T}."""
    t = int(math.isqrt(fr.n // 8))
    g = bipartite_graph(fr, movers)
    core, fib = [], {}
    for v in g:
        x = v[1]
        big = [p for p, e in primes_of(x, spf) if p > t]
        assert len(big) <= 1 and all(e == 1 for p, e in primes_of(x, spf) if p > t), (x, t)
        if big:
            fib.setdefault(big[0], []).append(v)
        else:
            core.append(v)
    return t, g, core, fib


def induced(g, vertices):
    vs = set(vertices)
    return {v: g[v] & vs for v in vs}


def treewidth_upper(g, rule="minfill"):
    """The width of a greedy elimination ordering (min-degree or min-fill, ties by degree then vertex)."""
    g = {v: set(ns) for v, ns in g.items()}
    width = 0

    def fill(v):
        ns = list(g[v])
        c = 0
        for i in range(len(ns)):
            gi = g[ns[i]]
            for j in range(i + 1, len(ns)):
                if ns[j] not in gi:
                    c += 1
        return c

    cache = {}
    dirty = set(g)
    while g:
        if rule == "mindegree":
            v = min(g, key=lambda u: (len(g[u]), u))
        else:
            for u in dirty:
                if u in g:
                    cache[u] = (fill(u), len(g[u]), u)
            dirty = set()
            v = min(cache.values())[2]
        ns = list(g[v])
        width = max(width, len(ns))
        for i in range(len(ns)):
            for j in range(i + 1, len(ns)):
                g[ns[i]].add(ns[j])
                g[ns[j]].add(ns[i])
        for w in ns:
            g[w].discard(v)
        del g[v]
        cache.pop(v, None)
        dirty.update(ns)
        for w in ns:
            dirty.update(g[w])
    return width


def treewidth_lower(g):
    """The minor-min-width lower bound: contract a vertex of least degree into its neighbor of least degree."""
    g = {v: set(ns) for v, ns in g.items()}
    lower = 0
    while len(g) > 1:
        v = min(g, key=lambda u: (len(g[u]), u))
        d = len(g[v])
        lower = max(lower, d)
        if d == 0:
            del g[v]
            continue
        w = min(g[v], key=lambda u: (len(g[u]), u))
        # contract v into w
        for x in g[v]:
            if x != w:
                g[x].discard(v)
                g[x].add(w)
                g[w].add(x)
        g[w].discard(v)
        del g[v]
    return lower
