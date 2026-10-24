#!/usr/bin/env python3
"""Draws the figures of docs/theory/ as SVG files in this folder.

Run it from the repository root with `.venv/bin/python docs/theory/img/draw.py`.  It needs only the standard library.
Every figure is one function with its data written out; the drawing helpers at the top give the figures one look:

* numbers (moves, sources, members) are circles and factors are squares; the ground vertex is a small filled square;
* a hollow shape was rejected or is not played, a gray one is kept, and a double outline marks a candidate;
* a distinction is made by line weight, line style, shape or label, with gray as a second cue only, so that the
  figures read the same to a reader who cannot tell red from green;
* each figure sits on a white rectangle, so that it reads on a dark page.
"""

import math
from html import escape
from pathlib import Path

OUT = Path(__file__).resolve().parent

FONT = "Helvetica, Arial, sans-serif"
INK = "#111111"
MID = "#666666"
LIGHT = "#b4b4b4"
FAINT = "#dcdcdc"
KEPT = "#cdcdcd"  # fill of a kept number
PAPER = "#ffffff"
CIRCLE_R = 16
SQUARE_H = 14
RING_GAP = 4.5
GAP = 1.5

# Line styles: (stroke, width, dash pattern, round caps).
STYLES = {
    "thin": (MID, 1.4, None, False),
    "thick": (INK, 3.6, None, False),
    "lightthick": (LIGHT, 3.6, None, False),  # a claim in a closed frame
    "dashed": (MID, 1.5, "6 4", False),
    "thickdashed": (INK, 3.2, "10 5", False),
    "dotted": (MID, 1.8, "1 4.5", True),
    "thickdotted": (INK, 3.2, "1 5.5", True),
    "faint": (LIGHT, 1.3, None, False),
    "faintdashed": (LIGHT, 1.4, "6 4", False),
    "claimout": (LIGHT, 3.4, "8 5", False),  # a claim that has been taken out of its frame
    "pay": (INK, 3.2, None, False),
    "order": (INK, 1.8, "7 4", False),
}


class Node:
    """A drawn circle or square: where edges attach, and how far from its center they stop."""

    def __init__(self, x, y, kind, size, ring=False):
        self.x, self.y, self.kind, self.size, self.ring = x, y, kind, size, ring

    def reach(self, ux, uy):
        """Distance from the center to the outline along the unit vector (ux, uy), plus a gap."""
        d = self.size if self.kind == "circle" else self.size / max(abs(ux), abs(uy), 1e-9)
        return d + (RING_GAP if self.ring else 0) + GAP

    @property
    def pos(self):
        return (self.x, self.y)


def _unit(dx, dy):
    n = math.hypot(dx, dy) or 1.0
    return dx / n, dy / n


def _fmt(v):
    return f"{v:.1f}".rstrip("0").rstrip(".")


class Fig:
    """An SVG figure: layers of shapes on a white rectangle."""

    def __init__(self, name, width, height, alt):
        self.name, self.width, self.height, self.alt = name, width, height, alt
        self.layers = {"box": [], "edge": [], "node": [], "text": []}
        self.texts = []  # (x, anchor, size, weight, string), to check that no line runs off the figure

    # -- plumbing ---------------------------------------------------------------------------------------------

    def add(self, layer, svg):
        self.layers[layer].append(svg)

    def svg(self):
        head = (
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{self.width}" height="{self.height}" '
            f'viewBox="0 0 {self.width} {self.height}" font-family="{FONT}" font-size="14">\n'
            f"<title>{escape(self.alt)}</title>\n"
            f'<rect x="0" y="0" width="{self.width}" height="{self.height}" fill="{PAPER}"/>\n'
        )
        body = "\n".join(s for layer in ("box", "edge", "node", "text") for s in self.layers[layer])
        return head + body + "\n</svg>\n"

    def check_text(self):
        """Warns about a line of text that might run off the figure.  Fonts differ from machine to machine, so a
        character is taken to be as wide as 0.6 (0.66 in bold) of the font size, wider than Helvetica's."""
        for x, anchor, size, weight, s in self.texts:
            w = len(s) * size * (0.66 if weight == "bold" else 0.6)
            left = {"start": x, "middle": x - w / 2, "end": x - w}[anchor]
            if left < 4 or left + w > self.width - 4:
                print(f"warning: {self.name}: the line {s!r} may run off the figure")

    def save(self):
        self.check_text()
        (OUT / f"{self.name}.svg").write_text(self.svg(), encoding="utf-8")

    # -- text -------------------------------------------------------------------------------------------------

    def text(
        self, x, y, s, size=14, anchor="middle", weight="normal", fill=INK, italic=False, halo=False, layer="text"
    ):
        """Text with its middle line at y.  halo puts a white outline behind it, for a label on an edge."""
        style = ' font-style="italic"' if italic else ""
        extra = f' stroke="{PAPER}" stroke-width="4" stroke-linejoin="round" paint-order="stroke"' if halo else ""
        self.texts.append((x, anchor, size, weight, s))
        self.add(
            layer,
            f'<text x="{_fmt(x)}" y="{_fmt(y)}" text-anchor="{anchor}" dy=".35em" font-size="{size}" '
            f'font-weight="{weight}" fill="{fill}"{style}{extra}>{escape(s)}</text>',
        )

    def lines(self, x, y, rows, size=13, gap=18, **kw):
        """Several lines of text, one under another, the first with its middle line at y."""
        for i, row in enumerate(rows):
            self.text(x, y + i * gap, row, size=size, **kw)

    def title(self, x, y, s, anchor="middle"):
        """A panel title."""
        self.text(x, y, s, size=15, weight="bold", anchor=anchor)

    # -- nodes ------------------------------------------------------------------------------------------------

    def number(self, x, y, label, r=CIRCLE_R, kept=False, hollow=False, ring=False, bold=False, fs=14, hidden=False):
        """A number (a move or a source) as a circle: gray when kept, hollow with a dashed outline when rejected."""
        fill = KEPT if kept else PAPER
        dash = ' stroke-dasharray="4 3"' if hollow else ""
        sw = 2.8 if bold else 1.7
        stroke = LIGHT if hidden else INK
        if ring:
            self.add(
                "node",
                f'<circle cx="{_fmt(x)}" cy="{_fmt(y)}" r="{_fmt(r + RING_GAP)}" fill="{PAPER}" stroke="{stroke}" '
                f'stroke-width="1.4"{dash}/>',
            )
        self.add(
            "node",
            f'<circle cx="{_fmt(x)}" cy="{_fmt(y)}" r="{_fmt(r)}" fill="{fill}" stroke="{stroke}" '
            f'stroke-width="{sw}"{dash}/>',
        )
        self.text(x, y, str(label), size=fs, fill=LIGHT if hidden else INK, layer="node")
        return Node(x, y, "circle", r, ring)

    def factor(self, x, y, label, h=SQUARE_H, kept=False, hollow=False, ring=False, bold=False, fs=14, hidden=False):
        """A factor as a square; ring draws the double outline of a candidate."""
        fill = KEPT if kept else PAPER
        dash = ' stroke-dasharray="4 3"' if hollow else ""
        sw = 3.2 if bold else 1.7
        stroke = LIGHT if hidden else INK
        if ring:
            o = h + RING_GAP
            self.add(
                "node",
                f'<rect x="{_fmt(x - o)}" y="{_fmt(y - o)}" width="{_fmt(2 * o)}" height="{_fmt(2 * o)}" '
                f'fill="{PAPER}" stroke="{stroke}" stroke-width="1.4"{dash}/>',
            )
        self.add(
            "node",
            f'<rect x="{_fmt(x - h)}" y="{_fmt(y - h)}" width="{_fmt(2 * h)}" height="{_fmt(2 * h)}" '
            f'fill="{fill}" stroke="{stroke}" stroke-width="{sw}"{dash}/>',
        )
        self.text(x, y, str(label), size=fs, fill=LIGHT if hidden else INK, layer="node")
        return Node(x, y, "square", h, ring)

    def ground(self, x, y, label="ground", side="below"):
        """The ground vertex: a small filled square with its name beside it."""
        h = 7
        self.add("node", f'<rect x="{_fmt(x - h)}" y="{_fmt(y - h)}" width="{2 * h}" height="{2 * h}" fill="{INK}"/>')
        if label:
            dy = {"below": 19, "above": -19}[side]
            self.text(x, y + dy, label, size=12, fill=MID)
        return Node(x, y, "square", h)

    def tag(self, x, y, label, shape="round", size=9):
        """A small numbered tag on an edge: round for a back move, square for a front move."""
        if shape == "round":
            self.add(
                "node",
                f'<circle cx="{_fmt(x)}" cy="{_fmt(y)}" r="{size}" fill="{PAPER}" stroke="{INK}" stroke-width="1.5"/>',
            )
        else:
            self.add(
                "node",
                f'<rect x="{_fmt(x - size)}" y="{_fmt(y - size)}" width="{2 * size}" height="{2 * size}" '
                f'fill="{PAPER}" stroke="{INK}" stroke-width="1.5"/>',
            )
        self.text(x, y, str(label), size=11, weight="bold", layer="node")

    # -- edges ------------------------------------------------------------------------------------------------

    def _style(self, style):
        stroke, width, dash, round_cap = STYLES[style]
        attrs = f'stroke="{stroke}" stroke-width="{width}" fill="none"'
        if dash:
            attrs += f' stroke-dasharray="{dash}"'
        if round_cap:
            attrs += ' stroke-linecap="round"'
        return stroke, width, attrs

    def _head(self, tip, direction, color, size=11):
        ux, uy = direction
        bx, by = tip[0] - ux * size, tip[1] - uy * size
        px, py = -uy * size * 0.42, ux * size * 0.42
        pts = f"{_fmt(tip[0])},{_fmt(tip[1])} {_fmt(bx + px)},{_fmt(by + py)} {_fmt(bx - px)},{_fmt(by - py)}"
        self.add("edge", f'<polygon points="{pts}" fill="{color}"/>')

    def edge(self, a, b, style="thin", head=None, tail=None):
        """A straight edge between two nodes, stopped at their outlines.  head/tail: an arrowhead at b / at a."""
        ux, uy = _unit(b.x - a.x, b.y - a.y)
        p = (a.x + ux * a.reach(ux, uy), a.y + uy * a.reach(ux, uy))
        q = (b.x - ux * b.reach(ux, uy), b.y - uy * b.reach(ux, uy))
        color, width, attrs = self._style(style)
        if tail:
            p = (p[0] + ux * 9, p[1] + uy * 9)
        if head:
            q = (q[0] - ux * 9, q[1] - uy * 9)
        self.add("edge", f'<line x1="{_fmt(p[0])}" y1="{_fmt(p[1])}" x2="{_fmt(q[0])}" y2="{_fmt(q[1])}" {attrs}/>')
        if head:
            self._head((q[0] + ux * 9, q[1] + uy * 9), (ux, uy), color)
        if tail:
            self._head((p[0] - ux * 9, p[1] - uy * 9), (-ux, -uy), color)

    def curve(self, a, b, bend, style="dotted", head=None, tail=None, control=None):
        """A quadratic curve between two nodes.  bend is the control point's offset to the left of a-to-b (y points
        down, so a positive bend bows to the right of the direction of travel); control gives the point itself."""
        mx, my = (a.x + b.x) / 2, (a.y + b.y) / 2
        ux, uy = _unit(b.x - a.x, b.y - a.y)
        c = control if control else (mx - uy * bend, my + ux * bend)
        sx, sy = _unit(c[0] - a.x, c[1] - a.y)
        ex, ey = _unit(b.x - c[0], b.y - c[1])
        p = (a.x + sx * a.reach(sx, sy), a.y + sy * a.reach(sx, sy))
        q = (b.x - ex * b.reach(ex, ey), b.y - ey * b.reach(ex, ey))
        color, width, attrs = self._style(style)
        if tail:
            p = (p[0] + sx * 9, p[1] + sy * 9)
        if head:
            q = (q[0] - ex * 9, q[1] - ey * 9)
        self.add(
            "edge",
            f'<path d="M{_fmt(p[0])},{_fmt(p[1])} Q{_fmt(c[0])},{_fmt(c[1])} {_fmt(q[0])},{_fmt(q[1])}" {attrs}/>',
        )
        if head:
            self._head((q[0] + ex * 9, q[1] + ey * 9), (ex, ey), color)
        if tail:
            self._head((p[0] - sx * 9, p[1] - sy * 9), (-sx, -sy), color)
        return c

    def cubic(self, p0, p1, p2, p3, style="dotted", head=False, tail=False):
        """A cubic curve from p0 to p3 with control points p1 and p2, with an arrowhead at either end whose tip is at
        the end point."""
        t0, t3 = _unit(p1[0] - p0[0], p1[1] - p0[1]), _unit(p3[0] - p2[0], p3[1] - p2[1])
        a = (p0[0] + t0[0] * 9, p0[1] + t0[1] * 9) if tail else p0
        b = (p3[0] - t3[0] * 9, p3[1] - t3[1] * 9) if head else p3
        color, width, attrs = self._style(style)
        pts = " ".join(f"{_fmt(p[0])},{_fmt(p[1])}" for p in (p1, p2, b))
        self.add("edge", f'<path d="M{_fmt(a[0])},{_fmt(a[1])} C{pts}" {attrs}/>')
        if head:
            self._head(p3, t3, color)
        if tail:
            self._head(p0, (-t0[0], -t0[1]), color)

    # -- boxes ------------------------------------------------------------------------------------------------

    def box(self, x, y, w, h, dashed=False, stroke=MID, fill="none", rx=12, width=1.6):
        """A rounded rectangle: a frame (dashed when it is closed or outside the problem)."""
        dash = ' stroke-dasharray="7 5"' if dashed else ""
        self.add(
            "box",
            f'<rect x="{_fmt(x)}" y="{_fmt(y)}" width="{_fmt(w)}" height="{_fmt(h)}" rx="{rx}" fill="{fill}" '
            f'stroke="{stroke}" stroke-width="{width}"{dash}/>',
        )


def midpoint(a, b, t=0.5):
    """The point a fraction t of the way from a to b (tuples)."""
    return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)


# ---------------------------------------------------------------------------------------------------------------
# Game 21's square: the primes 2, 5, 3, 7 sit at the corners in that cyclic order, and a number that is the product of
# two of them sits on the edge between them.  The figures of games' squares all use this one layout.
# ---------------------------------------------------------------------------------------------------------------

CORNERS = {2: (-1, -1), 5: (1, -1), 3: (1, 1), 7: (-1, 1)}  # the primes, as signs of the offset from the center
SIDES = {10: (2, 5), 15: (5, 3), 21: (3, 7), 14: (7, 2), 6: (2, 3)}  # each number as the pair of primes it joins


def square21(fig, cx, cy, s, numbers=(10, 15, 21, 14), kept=(), hollow=(), fs=14, r=CIRCLE_R + 1):
    """Draws game 21's square of primes with the given numbers on its edges.  Returns {prime or number: Node}."""
    nodes = {}
    for p, (sx, sy) in CORNERS.items():
        nodes[p] = fig.factor(cx + sx * s, cy + sy * s, p)
    for m in numbers:
        a, b = SIDES[m]
        x = (nodes[a].x + nodes[b].x) / 2
        y = (nodes[a].y + nodes[b].y) / 2
        nodes[m] = fig.number(x, y, m, r=r, kept=m in kept, hollow=m in hollow, fs=fs)
    return nodes


# ---------------------------------------------------------------------------------------------------------------
# Figures
# ---------------------------------------------------------------------------------------------------------------


def cycle_21():
    """The cycle of 10, 14, 15, 21 on the primes 2, 5, 3, 7, with the two payment schemes and their precedences."""
    fig = Fig(
        "cycle-21",
        760,
        430,
        "The numbers 10, 14, 15 and 21 as the four edges of a square on the primes 2, 5, 3 and 7; "
        "each of the two payment schemes pays every number with an end of its edge, and its precedence is a cycle.",
    )
    s = 105
    # scheme -> (title, what each number pays with, the precedence cycle)
    schemes = {
        "A": ({10: 2, 14: 7, 21: 3, 15: 5}, [10, 14, 21, 15]),
        "B": ({10: 5, 15: 3, 21: 7, 14: 2}, [10, 15, 21, 14]),
    }
    for (name, (pay, cycle)), cx in zip(schemes.items(), (190, 570)):
        cy = 205
        pays = ", ".join(f"{m}:{pay[m]}" for m in sorted(pay, key=cycle.index))
        fig.title(cx, 28, f"Scheme {name}: {pays}")
        nodes = square21(fig, cx, cy, s)
        for m in (10, 15, 21, 14):
            for prime in SIDES[m]:
                if pay[m] == prime:
                    fig.edge(nodes[m], nodes[prime], "pay", head=True)
                else:
                    fig.edge(nodes[m], nodes[prime], "thin")
        for a, b in zip(cycle, cycle[1:] + cycle[:1]):
            fig.edge(nodes[a], nodes[b], "order", head=True)
            # the reason for the edge: a's payment divides b
            mx, my = midpoint(nodes[a].pos, nodes[b].pos)
            nx, ny = _unit(nodes[b].y - nodes[a].y, nodes[a].x - nodes[b].x)  # a normal of the arrow
            if (cx - mx) * nx + (cy - my) * ny < 0:
                nx, ny = -nx, -ny  # the one that points into the square
            fig.text(mx + nx * 24, my + ny * 24, f"{pay[a]} | {b}", size=12, fill=MID)
    fig.lines(
        380,
        352,
        [
            "Thick arrow: the prime that a number pays with.",
            "Dashed arrow a → b: a is played before b, since a's payment divides b",
            "(the label 2 | 14 says that 2 divides 14).",
            "In each scheme the dashed arrows form a cycle, so no order of play exists.",
        ],
        size=13,
    )
    fig.save()


def graphic_21():
    """The graphic matroid of the edges 6, 10, 14, 15, 21 on the primes 2, 3, 5, 7, and the scan that keeps 21, 15, 14"""
    fig = Fig(
        "graphic-21",
        760,
        400,
        "The numbers 6, 10, 14, 15 and 21 as edges on the primes 2, 3, 5 and 7: the scan from the heaviest keeps "
        "21, 15 and 14 and rejects 10 and 6, which would each close a cycle.",
    )
    cx, cy, s = 175, 195, 105
    fig.title(cx, 30, "The edges, and what the scan keeps")
    nodes = square21(fig, cx, cy, s, numbers=(10, 15, 21, 14, 6), kept=(21, 15, 14), hollow=(10, 6))
    for m, (a, b) in SIDES.items():
        style = "thick" if m in (21, 15, 14) else "dashed"
        fig.edge(nodes[a], nodes[m], style)
        fig.edge(nodes[m], nodes[b], style)
    # the scan, as a table
    x0 = 392
    fig.title(x0, 30, "The scan, from the heaviest", anchor="start")
    rows = [
        (21, "keep", "3–7"),
        (15, "keep", "3–5"),
        (14, "keep", "2–7"),
        (10, "reject", "closes the cycle 10, 14, 15, 21"),
        (6, "reject", "closes the triangle 6, 14, 21"),
    ]
    for i, (m, verdict, why) in enumerate(rows):
        y = 78 + i * 52
        keep = verdict == "keep"
        fig.number(x0 + 20, y, m, kept=keep, hollow=not keep)
        fig.text(x0 + 52, y, verdict, anchor="start", weight="bold" if keep else "normal")
        fig.text(x0 + 108, y, why, anchor="start", size=13, fill=MID)
    fig.lines(x0, 348, ["Kept: 21 + 15 + 14 = 50, the heaviest", "of the 8 bases."], size=13, anchor="start")
    fig.save()


def payment_graph_21():
    """The payment graph of 10, 14, 15, 21 against 2, 3, 5, 7: a matching that cannot be extended, and a perfect one."""
    fig = Fig(
        "payment-graph-21",
        760,
        470,
        "The eight-cycle joining 10, 14, 15 and 21 to the primes of their maximal factors, with a matching of three "
        "edges that cannot be extended and a perfect matching of four.",
    )
    s = 105
    panels = [
        ("I: a matching that cannot be extended", {(10, 2), (14, 7), (15, 3)}),
        ("A perfect matching", {(10, 2), (14, 7), (21, 3), (15, 5)}),
    ]
    for (title, matching), cx in zip(panels, (190, 570)):
        cy = 208
        fig.title(cx, 28, title)
        nodes = square21(fig, cx, cy, s)
        for m in (10, 15, 21, 14):
            for prime in SIDES[m]:
                fig.edge(nodes[m], nodes[prime], "thick" if (m, prime) in matching else "dashed")
    fig.text(190, 355, "21: no edge left", size=14, weight="bold")
    fig.lines(190, 385, ["No other edge can be added:", "each shares a number or a", "prime with the matching."])
    fig.lines(570, 385, ["Four edges: every number is paid,", "each with a different prime."], size=13)
    fig.lines(380, 450, ["Thick: the edges of the matching.   Dashed: the other edges."], size=13, fill=MID)
    fig.save()


# ---------------------------------------------------------------------------------------------------------------
# Game 21's frames, as in sources.md and frame-model.md.  The numbers' maximal factors are computed from the
# definition, so that no edge is drawn from memory.
# ---------------------------------------------------------------------------------------------------------------


def maximal_factors(x):
    """The maximal factors of x: x divided by each of its prime divisors."""
    return sorted({x // p for p in range(2, x + 1) if x % p == 0 and all(p % q for q in range(2, p))})


# (factors left to right, sources left to right) of each component of game 21's sources; a source sits above the
# middle of its factors, so that no edge of a component crosses another.
COMPONENTS_21 = [
    ((1,), (11, 13, 17, 19)),
    ((10, 4, 6, 9), (20, 12, 18)),
    ((2, 7, 3, 5), (14, 21, 15)),
    ((8,), (16,)),
]
USTAR_21 = {12, 14, 15, 16, 18, 19, 20, 21}  # the sources that descending greedy keeps
FAN = 44  # distance between sources that share a single factor


def _check_components(components):
    for factors, sources in components:
        for s in sources:
            assert set(maximal_factors(s)) <= set(factors), (s, factors)


_check_components(COMPONENTS_21)


def place_component(factors, sources, x0, d):
    """Positions (x of each factor, x of each source) of a component whose first factor is at x0 and whose factors
    are d apart.  Sources with several factors sit above the middle of their first and last factor; sources that
    share one factor fan out above it."""
    fx = {f: x0 + i * d for i, f in enumerate(factors)}
    sx = {}
    if len(factors) == 1:
        for i, s in enumerate(sources):
            sx[s] = x0 + (i - (len(sources) - 1) / 2) * FAN
    else:
        for s in sources:
            xs = [fx[f] for f in maximal_factors(s)]
            sx[s] = sum(xs) / len(xs)
    return fx, sx


def sources_21():
    """The incidence graph of game 21's sources in its four components, and a run of peel on U* beneath it."""
    fig = Fig(
        "sources-21",
        780,
        630,
        "The sources 11 to 21 of game 21 above the factors 1 to 10, joined to their maximal factors in four "
        "components; below, a run of peel on the sources U* keeps, numbered 1 to 8 on the pairs it makes.",
    )
    d = 52
    run = [  # the run of peel of playability.md: (source, factor, kind of move)
        (16, 8, "front"),
        (19, 1, "front"),
        (14, 2, "back"),
        (15, 5, "back"),
        (18, 9, "back"),
        (12, 6, "back"),
        (20, 4, "back"),
        (21, 3, "back"),
    ]
    for top, title, peel in ((0, "The sources, by component", False), (290, "A run of peel on U*", True)):
        y_src, y_fac = top + 90, top + 200
        fig.title(390, top + 28, title)
        pairs = {(s, f): (i + 1, kind) for i, (s, f, kind) in enumerate(run)}
        x = 36
        for factors, sources in COMPONENTS_21:
            width = max((len(factors) - 1) * d, (len(sources) - 1) * FAN if len(factors) == 1 else 0)
            fx, sx = place_component(factors, sources, x + 20 + (width / 2 if len(factors) == 1 else 0), d)
            fig.box(x - 6, y_src - 36, width + 52, y_fac - y_src + 72, stroke=FAINT, width=1.6)
            fnodes = {f: fig.factor(fx[f], y_fac, f) for f in factors}
            snodes = {}
            for s in sources:
                in_ustar = s in USTAR_21
                snodes[s] = fig.number(sx[s], y_src, s, kept=in_ustar, hollow=not in_ustar)
            for s in sources:
                for f in maximal_factors(s):
                    if peel and (s, f) in pairs:
                        k, kind = pairs[(s, f)]
                        fig.edge(snodes[s], fnodes[f], "thick")
                        mx, my = midpoint((sx[s], y_src), (fx[f], y_fac), 0.5)
                        fig.tag(mx, my, k, shape="square" if kind == "front" else "round")
                    elif peel and s not in USTAR_21:
                        fig.edge(snodes[s], fnodes[f], "dotted")
                    else:
                        fig.edge(snodes[s], fnodes[f], "faint" if peel else "thin")
            x += width + 52 + 22
        if peel:
            note = "11, 13 and 17 list only 1, which 19 took: nothing is left for them."
            fig.text(390, top + 262, note, size=13, fill=MID)
    fig.lines(
        390,
        596,
        ["Circles: sources, gray when U* keeps them.  Squares: factors.",
         "Numbers on the thick pairs: the order of the run.  Square tag: a front move.  Round tag: a back move."],
        size=12,
        fill=MID,
    )
    fig.height = 630
    fig.save()


# ---------------------------------------------------------------------------------------------------------------
# The frames of game 21 once more, as frames-21, promotion-21 and frame-matroids-21 draw them.  A frame's moves sit
# in a row above its factors, in the order of the components of sources.md.
# ---------------------------------------------------------------------------------------------------------------


def draw_frame(fig, moves, factors, y_move, y_fac, d, x0, rings=(), hollow_moves=(), hidden=(), styles=None):
    """Draws a frame: moves (label -> column of its x, in units of d) above factors (label -> column), with an edge
    from each move to each of its maximal factors that is a factor of the frame.  styles gives the style of an edge
    (move, factor), thin by default.  Returns {label: Node}."""
    styles = styles or {}
    nodes = {}
    for f, col in factors.items():
        nodes[f] = fig.factor(x0 + col * d, y_fac, f, ring=f in rings, hidden=f in hidden)
    for m, col in moves.items():
        nodes[m] = fig.number(
            x0 + col * d, y_move, m, kept=m not in hollow_moves, hollow=m in hollow_moves, ring=m in rings,
            hidden=m in hidden,
        )
    for m in moves:
        for f in maximal_factors(m):
            if f in factors:
                fig.edge(nodes[m], nodes[f], styles.get((m, f), "thin"))
    return nodes


def frames_21():
    """Game 21's four frames stacked by layer, with their claims, candidates and in-front-of links."""
    fig = Fig(
        "frames-21",
        780,
        730,
        "Game 21's four frames, one box per layer with layer 4 at the top: sources above factors, the claims as thick "
        "pairs, the closed frames dashed, the candidates 4 and 9 with a double outline, and dotted arrows from a "
        "factor to its maximal factor in the frame in front.",
    )
    d, x0 = 60, 170
    box_h, y_s, y_f = 126, 32, 98
    # Each layer's factors and sources sit at columns of width d, so that a factor lies under the factor it divides:
    # 4 above 2, 9 above 3, 8 above 4, 2 above 1.  frame is the frame's number (None for a closed frame), and claim is
    # the pair that setup claims, with its kind.  Layer 1's four sources fan out around its one factor.
    fan = [(s, 1 + k * FAN / d) for s, k in zip((11, 13, 17, 19), (-1.5, -0.5, 0.5, 1.5))]
    layers = {  # layer -> top of its box, and its frame
        4: dict(top=40, factors={8: 1}, moves={16: 1}, claim=(16, 8, "front")),
        3: dict(top=196, factors={10: 0, 4: 1, 6: 2, 9: 3}, moves={20: 0.5, 12: 1.5, 18: 2.5}, claim=(20, 10, "back")),
        2: dict(top=352, factors={2: 1, 7: 2, 3: 3, 5: 4}, moves={14: 1.5, 21: 2.5, 15: 3.5}, claim=(15, 5, "back")),
        1: dict(top=508, factors={1: 1}, moves=dict(fan), claim=(19, 1, "back")),
    }
    candidates = {4, 9}
    frame_number = 0  # frames with freedoms are numbered from the lowest layer up
    nodes = {}
    for layer in sorted(layers):
        spec = layers[layer]
        kept = {m for m in spec["moves"] if m in USTAR_21}
        spec["freedoms"] = len(spec["factors"]) - len(kept)  # the factors of the frame less the moves U* keeps in it
        spec["frame"] = None if spec["freedoms"] == 0 else frame_number
        frame_number += spec["freedoms"] > 0
    for layer, spec in layers.items():
        closed = spec["frame"] is None
        xs = [x0 + c * d for c in list(spec["factors"].values()) + list(spec["moves"].values())]
        left, right = min(xs) - 40, max(xs) + 40
        fig.box(left, spec["top"], right - left, box_h, dashed=closed, stroke=MID if closed else INK)
        s, f, kind = spec["claim"]
        nodes.update(
            draw_frame(
                fig, spec["moves"], spec["factors"], spec["top"] + y_s, spec["top"] + y_f, d, x0,
                rings=candidates & set(spec["factors"]), hollow_moves={m for m in spec["moves"] if m not in USTAR_21},
                styles={(s, f): "lightthick" if closed else "thick"},
            )
        )
        # a name for each box, on its right
        title = f"Layer {layer}" + ("" if closed else f": frame {spec['frame']}")
        rows = ["closed: no freedom" if closed else f"{spec['freedoms']} freedom", f"{kind} claim ({s}, {f})"]
        if candidates & set(spec["factors"]):
            rows.append("candidates " + " and ".join(map(str, sorted(candidates))))
        fig.text(480, spec["top"] + 24, title, anchor="start", weight="bold")
        fig.lines(480, spec["top"] + 46, rows, size=13, anchor="start", fill=MID)
    # "in front of": a factor to its maximal factor one box down
    for upper, lower in ((9, 3), (4, 2), (8, 4), (2, 1)):
        assert lower in maximal_factors(upper)
        fig.edge(nodes[upper], nodes[lower], "dotted", head=True)
    fig.lines(
        390,
        680,
        [
            "Circles: sources, gray where descending greedy keeps them.  Squares: factors.",
            "Double outline: candidates.  Thick pair: a claim, lighter in a closed frame.",
            "Dotted arrow: a factor to its maximal factor in the frame in front.",
        ],
        size=12,
        fill=MID,
    )
    fig.save()


def promotion_21():
    """Game 21's two root frames before and after promoting 9, with the drops that empty them."""
    fig = Fig(
        "promotion-21",
        780,
        560,
        "Game 21's root frames 1 (moves 12, 18) and 0 (moves 14, 21) before and after promoting the candidate 9, "
        "which leaves frame 1's factors and becomes a move of frame 0 paid by 3; five numbered drops empty both "
        "frames.",
    )
    d = 70
    frame1 = dict(factors={4: 0, 6: 1, 9: 2}, moves={12: 0.5, 18: 1.5})
    frame0 = dict(factors={2: 0, 7: 1, 3: 2}, moves={14: 0.5, 21: 1.5})
    y1m, y1f, y0m, y0f = 130, 205, 330, 405
    for name, x0, after in (("The root frames", 70, False), ("After promoting 9", 440, True)):
        fig.title(x0 + 105, 28, name)
        # frame boxes
        for label, top in (("Frame 1", 78), ("Frame 0", 278)):
            fig.box(x0 - 45, top, 300, 176, stroke=INK)
            fig.text(x0 - 31, top + 20, label, anchor="start", weight="bold")
        # frame 1: 9 stays as a ghost where it was
        moves1, factors1 = dict(frame1["moves"]), dict(frame1["factors"])
        n1 = draw_frame(
            fig, moves1, factors1, y1m, y1f, d, x0, rings={9} if after else {4, 9}, hidden={9} if after else (),
            styles={(18, 9): "faintdashed"} if after else None,
        )
        moves0, factors0 = dict(frame0["moves"]), dict(frame0["factors"])
        if after:
            moves0[9] = 2.5
        n0 = draw_frame(fig, moves0, factors0, y0m, y0f, d, x0, rings={9} if after else ())
        if not after:
            fig.edge(n1[9], n0[3], "dotted", head=True)
        else:
            fig.edge(n1[9], n0[9], "dotted", head=True)
            drops1 = [(18, 6), (12, 4)]
            drops0 = [(9, 3), (21, 7), (14, 2)]
            for k, (m, f) in enumerate(drops1, 1):
                fig.edge(n1[m], n1[f], "thick")
                fig.tag((n1[m].x + n1[f].x) / 2, (n1[m].y + n1[f].y) / 2, k, shape="square")
            for k, (m, f) in enumerate(drops0, 1):
                fig.edge(n0[m], n0[f], "thick")
                fig.tag((n0[m].x + n0[f].x) / 2, (n0[m].y + n0[f].y) / 2, k, shape="square")
            fig.text(x0 - 31, 78 + 156, "empty after the drops", size=13, fill=MID, anchor="start")
            fig.text(x0 - 31, 278 + 156, "empty after the drops", size=13, fill=MID, anchor="start")
    fig.lines(
        390,
        500,
        [
            "Promoting 9 moves it one frame forward: out of frame 1's factors, into frame 0's moves, paid by 3.",
            "Numbered square tags: the drops of each frame, in order, each a move with one live factor.",
            "The score goes from 135 to 144: the promoted 9 adds its value.",
        ],
        size=13,
    )
    fig.save()


def frame_matroids_21():
    """Game 21's root frames before any promotion, with the candidates 4 and 9 touching frame 1 as factors and frame 0
    as movers."""
    fig = Fig(
        "frame-matroids-21",
        780,
        470,
        "Game 21's root frames 1 and 0, with the candidates 4 and 9 (double outline) as factors of frame 1 and "
        "dashed arrows to the factors 2 and 3 of frame 0, where they would be movers: each candidate touches one "
        "odd and one even frame.",
    )
    d = 84
    x0 = 110
    y1m, y1f, y0m, y0f = 112, 190, 322, 400
    fig.box(x0 - 60, 62, 360, 170, stroke=INK)
    fig.box(x0 - 60, 272, 360, 170, stroke=INK)
    fig.text(x0 + 290, 84, "Frame 1: B = N₁ (odd)", anchor="end", weight="bold")
    fig.text(x0 + 290, 294, "Frame 0: A = N₀ (even)", anchor="end", weight="bold")
    n1 = draw_frame(fig, {12: 0.5, 18: 1.5}, {4: 0, 6: 1, 9: 2}, y1m + 12, y1f, d, x0, rings={4, 9})
    n0 = draw_frame(fig, {14: 0.5, 21: 1.5}, {2: 0, 7: 1, 3: 2}, y0m + 12, y0f, d, x0)
    fig.edge(n1[4], n0[2], "dashed", head=True)
    fig.edge(n1[9], n0[3], "dashed", head=True)
    x1 = 440
    fig.text(x1, 112, "In frame 1, 4 and 9 are factors", anchor="start", weight="bold")
    fig.lines(
        x1,
        138,
        ["Unit vectors e₄ and e₉.  The moves span", "e₄ + e₆ and e₆ + e₉, so 4 and 9 are", "parallel in B: rank 1."],
        size=13,
        anchor="start",
        fill=MID,
    )
    fig.text(x1, 322, "In frame 0, they are movers", anchor="start", weight="bold")
    fig.lines(
        x1,
        348,
        ["Vectors e₂ and e₃.  The moves span", "e₂ + e₇ and e₃ + e₇, so 4 and 9 are", "parallel in A: rank 1."],
        size=13,
        anchor="start",
        fill=MID,
    )
    fig.lines(
        390,
        458,
        ["Dashed arrow: a candidate to the maximal factor that would pay for it in the frame in front."],
        size=12,
        fill=MID,
    )
    fig.save()


def ring_points(cx, cy, radius, n, start=-90):
    """n points on a circle, the first at the top (start is its angle in degrees), going clockwise."""
    angles = [math.radians(start + 360 * i / n) for i in range(n)]
    return [(cx + radius * math.cos(a), cy + radius * math.sin(a)) for a in angles]


# The cycle of game 98: numbers (even places) and factors (odd places) alternate, and each number's maximal factors
# are its two neighbors.
CYCLE_98 = [22, 2, 62, 31, 93, 3, 57, 19, 95, 5, 65, 13, 91, 7, 77, 11]


def _check_cycle_98():
    n = len(CYCLE_98)
    for i in range(0, n, 2):
        assert set(maximal_factors(CYCLE_98[i])) == {CYCLE_98[i - 1], CYCLE_98[(i + 1) % n]}, CYCLE_98[i]


_check_cycle_98()


def cycle_98():
    """The 16-gon of game 98 that test (b) keeps from hiding behind the claim (77, 11)."""
    fig = Fig(
        "cycle-98",
        780,
        470,
        "Game 98's sixteen-cornered cycle of eight numbers and eight factors: on the board each factor is listed twice "
        "and the cycle is a core; in the model the claim (77, 11) is taken out, 22 joins only 2, and the cycle opens "
        "into a path.",
    )
    n = len(CYCLE_98)
    for cx, in_model in ((195, False), (585, True)):
        cy = 238
        fig.title(cx, 28, "In the model" if in_model else "On the board")
        pts = ring_points(cx, cy, 138, n)
        nodes = []
        for i, label in enumerate(CYCLE_98):
            gone = in_model and label in (77, 11)
            x, y = pts[i]
            if i % 2 == 0:
                nodes.append(fig.number(x, y, label, kept=label != 22, ring=label == 22, hidden=gone))
            else:
                nodes.append(fig.factor(x, y, label, hidden=gone))
        for i in range(n):
            a, b = nodes[i], nodes[(i + 1) % n]
            pair = {CYCLE_98[i], CYCLE_98[(i + 1) % n]}
            if not in_model:
                fig.edge(a, b, "thin")
            elif pair == {77, 11}:
                fig.edge(a, b, "claimout")
            elif 77 in pair or 11 in pair:
                continue  # the claim took 77 and 11 out of the frame, with their edges
            else:
                fig.edge(a, b, "thin")
    fig.lines(
        195,
        238 - 20,
        ["8 numbers on 8 factors,", "each factor listed twice:", "a core, so U* + 22", "is not playable"],
        size=13,
        gap=18,
    )
    fig.lines(
        585,
        238 - 20,
        ["without 77 and 11,", "22 joins only 2:", "a path, which peel", "empties"],
        size=13,
        gap=18,
    )
    fig.lines(
        390,
        428,
        [
            "Circles: numbers.  Squares: factors.  Double outline: the candidate 22.",
            "Light dashed pair: the claim (77, 11), taken out.",
            "Only test (b) keeps 22 out: with 77 and 11 gone the model cannot see the cycle the board holds.",
        ],
        size=12,
        fill=MID,
    )
    fig.height = 490
    fig.save()


def omega(x):
    """The number of prime factors of x, counted with multiplicity."""
    count, p = 0, 2
    while x > 1:
        while x % p == 0:
            x //= p
            count += 1
        p += 1
    return count


def spread(ys, gap):
    """Moves the values of ys apart, keeping their order, until neighbors are at least gap apart."""
    order = sorted(range(len(ys)), key=lambda i: ys[i])
    out = list(ys)
    for a, b in zip(order, order[1:]):
        out[b] = max(out[b], out[a] + gap)
    shift = (sum(ys) - sum(out)) / len(ys)
    return [v + shift for v in out]


def parity_10():
    """The board of 10: each number's vector lands in one parity's box and its own coordinate in the other's."""
    fig = Fig(
        "parity-10",
        780,
        600,
        "The numbers 2 to 10 in the middle with the coordinates 1 to 5 in two boxes, E0 = {1, 4} on the left and "
        "E1 = {2, 3, 5} on the right: each number's solid lines to its maximal factors land in one box and its "
        "dashed line to its own coordinate in the other; the numbers above 5 have no own coordinate.",
    )
    n = 10
    best = {6, 7, 8, 9, 10}  # the heaviest set independent in both parity matroids
    order = [8, 9, 6, 4, 10, 7, 3, 2, 5]  # top to bottom: layers 3, 2, 1, in the order with the fewest crossings
    coords = {0: [4, 1], 1: [3, 2, 5]}  # the coordinates of each box, top to bottom
    xc, xs = 390, {0: 95, 1: 685}
    ys, y = {}, 82
    for i, s in enumerate(order):
        if i and omega(s) != omega(order[i - 1]):
            y += 24
        ys[s] = y
        y += 46
    # each number's lines: solid to its maximal factors, dashed to its own coordinate when it has one
    solid = {s: maximal_factors(s) for s in order}
    dashed = {s: s for s in order if s <= n // 2}
    # every coordinate sits at the average height of the numbers it touches, in the box of its parity
    cy = {}
    for parity, cs in coords.items():
        want = []
        for c in cs:
            touched = [ys[s] for s in order if c in solid[s] or dashed.get(s) == c]
            want.append(sum(touched) / len(touched))
        for c, v in zip(cs, spread(want, 52)):
            cy[c] = v
    for parity, cs in coords.items():
        top, bottom = min(cy[c] for c in cs) - 42, max(cy[c] for c in cs) + 42
        fig.box(xs[parity] - 40, top, 80, bottom - top, stroke=INK)
        name = "E₀: Ω even" if parity == 0 else "E₁: Ω odd"
        fig.text(xs[parity], top - 16, name, weight="bold")
    cnodes = {c: fig.factor(xs[omega(c) % 2], cy[c], c) for c in cy}
    snodes = {s: fig.number(xc, ys[s], s, kept=s in best, ring=s > n // 2, bold=s in best) for s in order}
    for s in order:
        for f in solid[s]:
            fig.edge(snodes[s], cnodes[f], "thin")
        if s in dashed:
            fig.edge(snodes[s], cnodes[s], "dashed")
    for k in (3, 2, 1):  # the layer of each group of numbers, in the gap above it
        first = next(s for s in order if omega(s) == k)
        fig.text(xc, ys[first] - 38, f"Ω = {k}", size=13, fill=MID, weight="bold", halo=True)
    fig.lines(
        390,
        560,
        [
            "Solid line: from a number to each maximal factor.",
            "Dashed line: from a number up to 5 to its own coordinate.",
            "Double outline: a number above 5, with no coordinate of its own.",
            "Gray and bold: the heaviest set independent in both matroids, {6, 7, 8, 9, 10}.",
        ],
        size=12,
        fill=MID,
    )
    fig.height = 640
    fig.save()


def game_28():
    """Game 28's root problem: frame 1 and frame 0, the candidates and their payer, and the exchange of 5 and 7."""
    fig = Fig(
        "game-28",
        780,
        470,
        "Game 28's root problem: frame 1 with the moves 20 and 28 and the candidates 4, 10 and 14, each paid by the "
        "factor 2 of frame 0; the exchange of the primes 5 and 7 swaps 20 with 28 and 10 with 14 and fixes 4 and 2; "
        "a closed frame stays outside the problem.",
    )
    x0, d = 290, 120
    y_move, y_fac, y_pay = 140, 220, 350
    fig.box(x0 - 200, 18, 400, 250, stroke=INK)
    fig.box(x0 - 200, 300, 400, 100, stroke=INK)
    fig.text(x0 - 186, 252, "Frame 1", anchor="start", weight="bold")
    fig.text(x0 - 186, 384, "Frame 0", anchor="start", weight="bold")
    f1 = draw_frame(fig, {20: -0.5, 28: 0.5}, {10: -1, 4: 0, 14: 1}, y_move, y_fac, d, x0, rings={4, 10, 14})
    two = fig.factor(x0, y_pay, 2)
    for c in (10, 4, 14):
        fig.edge(f1[c], two, "dashed", head=True)
    # the exchange of 5 and 7: 20 <-> 28 and 10 <-> 14, drawn as nested arcs over the top of the frame
    fig.curve(f1[20], f1[28], 0, "dotted", head=True, tail=True, control=(x0, y_move - 78))
    left, right = (x0 - d - 8, y_fac - 18), (x0 + d + 8, y_fac - 18)
    fig.cubic(left, (left[0] - 66, 10), (right[0] + 66, 10), right, "dotted", head=True, tail=True)
    fig.text(x0, 44, "exchange of 5 and 7", size=13, fill=MID)
    fig.text(x0 + 10, y_fac + 36, "fixed", size=13, fill=MID, italic=True, anchor="start")
    fig.text(x0 + 24, y_pay + 4, "fixed", size=13, fill=MID, italic=True, anchor="start")
    # a closed frame outside the problem, greyed: 15, 21 and 25 over 5, 3 and 7
    cx = 655
    fig.box(cx - 110, 100, 220, 170, dashed=True, stroke=LIGHT)
    fx = {5: cx - 55, 3: cx, 7: cx + 55}
    mx = {25: fx[5] - 28, 15: (fx[5] + fx[3]) / 2, 21: (fx[3] + fx[7]) / 2}
    nodes = {}
    for f, x in fx.items():
        nodes[f] = fig.factor(x, 220, f, hidden=True)
    for m, x in mx.items():
        nodes[m] = fig.number(x, 140, m, hidden=True)
    for m in mx:
        for f in maximal_factors(m):
            fig.edge(nodes[m], nodes[f], "faint")
    fig.text(cx, 82, "a closed frame", size=13, fill=MID, weight="bold")
    fig.lines(cx, 296, ["outside the problem:", "25 would go to 49,", "off the board"], size=12, fill=MID)
    fig.lines(
        390,
        438,
        [
            "Dashed arrows: each candidate to its payer.",
            "Dotted double arrows: 20 = 2²·5 ↔ 28 = 2²·7 and 10 = 2·5 ↔ 14 = 2·7.",
        ],
        size=12,
        fill=MID,
    )
    fig.save()


def state_28():
    """Game 28 with 14 promoted: the frames, the three drops, and the two empty frames that no candidate can join."""
    fig = Fig(
        "state-28",
        780,
        420,
        "Game 28 after promoting 14: 14 leaves frame 1 for frame 0, the drops (28, 4), (20, 10) and (14, 2) empty both "
        "frames, and 4 and 10 are gone, so no candidate can join 14.",
    )
    d = 84
    y1m, y1f, y0m, y0f = 125, 195, 285, 345
    for x0, title, after in ((205, "After promoting 14", False), (590, "state({14}): after the drops", True)):
        fig.title(x0, 26, title)
        fig.box(x0 - 150, 56, 300, 190, stroke=INK)
        fig.box(x0 - 150, 262, 300, 108, stroke=INK)
        fig.text(x0 - 136, 76, "Frame 1", anchor="start", weight="bold")
        fig.text(x0 - 136, 282, "Frame 0", anchor="start", weight="bold")
        if not after:
            n1 = draw_frame(
                fig, {20: -0.5, 28: 0.5}, {10: -1, 4: 0, 14: 1}, y1m, y1f, d, x0, hidden={14},
                styles={(28, 14): "faintdashed"},
            )
            n0 = draw_frame(fig, {14: 0.5}, {2: 0}, y0m, y0f, d, x0, rings={14})
            fig.edge(n1[14], n0[14], "dotted", head=True)
            for k, (m, f) in enumerate([(28, 4), (20, 10)], 1):
                fig.edge(n1[m], n1[f], "thick")
                fig.tag((n1[m].x + n1[f].x) / 2, (n1[m].y + n1[f].y) / 2, k, shape="square")
            fig.edge(n0[14], n0[2], "thick")
            fig.tag((n0[14].x + n0[2].x) / 2, (n0[14].y + n0[2].y) / 2, 3, shape="square")
        else:
            for f, col in ((10, -1), (4, 0)):
                fig.factor(x0 + col * d, y1f, f, hidden=True, hollow=True)
                fig.text(x0 + col * d, y1f + 30, "gone", size=12, fill=MID, italic=True)
            fig.text(x0, y1m + 4, "empty", size=14, fill=MID)
            fig.text(x0, y0m + 24, "empty", size=14, fill=MID)
    fig.lines(
        390,
        388,
        ["Numbered square tags: the drops (28, 4), (20, 10) and (14, 2), in order.",
         "4 and 10 go out with them, so no candidate is left to join 14."],
        size=12,
        fill=MID,
    )
    fig.save()


def hypergraph_panel(fig, oy, members, factors, edges, styles, ground, title, notes, r=21):
    """One panel of a picture of sources as edges on factors: the factors (squares), the ground, and each source as a
    circle joined to its maximal factors, so that a source with three factors is a small hub with three spokes.
    styles maps each source to (circle options, edge style).  Returns the y below the panel's notes."""
    top = oy + 70  # the factors' y values are measured from here
    fig.title(390, oy + 22, title)
    nodes = {}
    for f, (x, y) in factors.items():
        nodes[f] = fig.factor(x, top + y, f, h=18, fs=13)
    gx, gy = ground
    nodes["ground"] = fig.ground(gx, top + gy, side="above")
    for m, (x, y) in members.items():
        nodes[m] = fig.number(x, top + y, m, r=r, fs=12.5, **styles[m][0])
    for m, ends in edges.items():
        for e in ends:
            fig.edge(nodes[m], nodes[e], styles[m][1])
    bottom = top + max(y for _, y in factors.values()) + 18
    for i, row in enumerate(notes):
        fig.text(390, bottom + 30 + i * 18, row, size=13, fill=MID)
    return nodes, bottom + 30 + len(notes) * 18


def exchange_2873():
    """The six sources of game 2873 as edges on the factors 121 to 221: the maximal playable sets A and B."""
    fig = Fig(
        "exchange-2873",
        780,
        640,
        "Game 2873's six sources as edges on the factors 121, 143, 169, 187, 221 and the ground, with the source 2431 "
        "joined to three factors: the playable set A of four sources that nothing can join, and the playable set B of "
        "five.",
    )
    # factor -> (x, y); each source sits on the edge between its factors (2431 is a hub with three spokes)
    factors = {121: (110, 0), 143: (270, 0), 169: (430, 0), 187: (110, 150), 221: (430, 150)}
    ground = (610, 0)
    members = {1573: (190, 0), 1859: (350, 0), 2197: (520, 0), 2057: (110, 75), 2873: (430, 75), 2431: (270, 105)}
    edges = {
        1573: (121, 143),
        1859: (143, 169),
        2057: (121, 187),
        2197: (169, "ground"),
        2873: (169, 221),
        2431: (143, 187, 221),
    }
    for m, ends in edges.items():
        assert set(maximal_factors(m)) == {x for x in ends if x != "ground"}, m  # the table of sources.md
    panels = [  # (set, title, notes)
        (
            {1573, 2057, 2431, 2873},
            "A = {1573, 2057, 2431, 2873}: playable, stuck at four",
            ["1859 would make the core {1573, 1859, 2057, 2431, 2873}, and 2197 would make a core by itself:",
             "no member of B fits into A."],
        ),
        (
            {1573, 1859, 2057, 2197, 2873},
            "B = {1573, 1859, 2057, 2197, 2873}: playable, with five",
            ["So the playable sets are not the independent sets of a matroid:",
             "one maximal set has four members and another five."],
        ),
    ]
    oy = 0
    for chosen, title, notes in panels:
        styles = {m: (dict(kept=True), "thick") if m in chosen else (dict(hollow=True), "dashed") for m in members}
        _, oy = hypergraph_panel(fig, oy, members, factors, edges, styles, ground, title, notes)
        oy += 16
    legend = "Circles: sources, each joined to its maximal factors (squares).  Solid: in the set.  Dashed: not in it."
    fig.text(390, oy + 6, legend, size=12, fill=MID)
    fig.height = oy + 26
    fig.save()


def cores_9170():
    """The six sources of game 9170 as edges on the factors 289 to 437: the two minimal cores, then a third."""
    fig = Fig(
        "cores-9170",
        780,
        800,
        "Game 9170's six sources as edges on the factors 289, 323, 361, 391, 437 and the ground, with the source 7429 "
        "joined to three factors: the two minimal cores of T and 6137 share all but 4913 and 5491, and with 6859 "
        "added a third core appears that does not hold 6137.",
    )
    factors = {289: (110, 170), 323: (390, 128), 361: (670, 170), 391: (190, 246), 437: (590, 246)}
    ground = (390, 0)
    members = {
        4913: (250, 85), 5491: (236, 151), 6137: (544, 151), 6647: (150, 208), 8303: (630, 208), 7429: (390, 208),
    }
    edges = {
        4913: (289, "ground"),
        5491: (289, 323),
        6137: (323, 361),
        6647: (289, 391),
        8303: (361, 437),
        7429: (323, 391, 437),
    }
    for m, ends in edges.items():
        assert set(maximal_factors(m)) == {x for x in ends if x != "ground"}, m  # the table of sources.md
    # panel 1: both minimal cores of T + 6137; where they differ, 4913 and 5491, two line styles tell them apart
    styles = {m: (dict(kept=True), "thick") for m in members}
    styles[4913] = (dict(kept=True), "thickdashed")
    styles[5491] = (dict(kept=True), "thickdotted")
    styles[6137] = (dict(kept=True, bold=True), "thick")
    _, oy = hypergraph_panel(
        fig, 0, members, factors, edges, styles, ground, "T and 6137: two minimal cores",
        ["Both cores hold d = 6137, 6647, 7429 and 8303.  Long dashes: the core with 4913.  Dots: with 5491.",
         "Removing 6137 breaks both cores; removing 4913 or 5491 leaves the other."],
        r=20,
    )
    fig.text(members[6137][0] + 32, 70 + members[6137][1] - 31, "d", size=15, weight="bold")
    # panel 2: add 6859 (361 to the ground), which closes a third core that does not hold 6137
    members2 = dict(members)
    members2[6859] = (530, 85)
    edges2 = dict(edges)
    edges2[6859] = (361, "ground")
    assert set(maximal_factors(6859)) == {361}
    core = {5491, 6647, 6859, 7429, 8303}
    styles2 = {m: (dict(kept=True), "thick") if m in core else (dict(hollow=True), "faint") for m in members2}
    styles2[6859] = (dict(kept=True), "thickdashed")
    _, oy = hypergraph_panel(
        fig, oy + 16, members2, factors, edges2, styles2, ground,
        "With 6859 added: the core {5491, 6647, 6859, 7429, 8303}",
        ["6859 = 19³ lies above 6137 and descending greedy keeps it, so a set that agrees with U* above 6137",
         "cannot hold 5491, and the second core of the first panel is out of its reach."],
        r=20,
    )
    legend = "Circles: sources, each joined to its maximal factors (squares).  Thick: in a core.  Hollow: outside it."
    fig.text(390, oy + 6, legend, size=12, fill=MID)
    fig.height = oy + 26
    fig.save()


def gf3_45():
    """The odd core {12, 18, 20, 30, 45} of game 45 with its GF(3) signing: each factor's values sum to 0."""
    fig = Fig(
        "gf3-45",
        780,
        500,
        "The five numbers 12, 18, 20, 30 and 45 as members joined to the factors 4, 6, 9, 10 and 15, with a value of "
        "1 or 2 on each edge so that the values at every factor add up to 0 modulo 3.",
    )
    ring = [12, 4, 20, 10, 30, 15, 45, 9, 18, 6]  # members and factors alternate; 6 is also joined to 30
    values = {(12, 4): 1, (4, 20): 2, (20, 10): 1, (10, 30): 2, (30, 15): 1, (15, 45): 2, (45, 9): 2, (9, 18): 1,
              (18, 6): 1, (6, 12): 1, (30, 6): 1}
    for a, b in values:
        assert a in maximal_factors(b) or b in maximal_factors(a), (a, b)
    cx, cy, radius = 260, 255, 168
    fig.title(390, 26, "The odd core {12, 18, 20, 30, 45} with a signing over GF(3)")
    pts = ring_points(cx, cy, radius, len(ring))
    nodes = {}
    for label, (x, y) in zip(ring, pts):
        nodes[label] = fig.number(x, y, label) if label in (12, 18, 20, 30, 45) else fig.factor(x, y, label)
    for (a, b), v in values.items():
        fig.edge(nodes[a], nodes[b], "thin")
        mx, my = midpoint(nodes[a].pos, nodes[b].pos, 0.5 if (a, b) != (30, 6) else 0.3)
        fig.tag(mx, my, v, shape="round", size=10)
    # each factor's sum, outside the ring
    for f, rowtext in {4: "1 + 2", 6: "1 + 1 + 1", 9: "1 + 2", 10: "1 + 2", 15: "1 + 2"}.items():
        x, y = nodes[f].pos
        ux, uy = _unit(x - cx, y - cy)
        fig.text(x + ux * 54, y + uy * 30, rowtext + " = 0", size=13, anchor="middle")
    fig.lines(
        615,
        150,
        ["Each edge carries the entry that", "its member has at that factor.", "At every factor the entries add",
         "up to 0 (mod 3), so the five", "vectors sum to the zero vector:", "they are dependent over GF(3),",
         "though independent over GF(2),", "where every entry is 1."],
        size=13,
    )
    fig.save()


def extended_24():
    """Game 24 with O = {16, 20, 24}: promoting 8 and 12 into frame 1 works unless the claim (18, 6) is kept."""
    fig = Fig(
        "extended-24",
        780,
        400,
        "Frame 1 of game 24 with the numbers 8 and 12 promoted into it: with the move 18 and the factors 4, 6, 9 and "
        "10 the frame is winnable, but with the claim (18, 6) kept, 8 and 12 both need the factor 4.",
    )
    d = 78
    for x0, title, kept_claim in ((95, "Without the claim", False), (480, "With the claim (18, 6) kept", True)):
        fig.title(x0 + 110, 28, title)
        fig.box(x0 - 70, 52, 360, 230, stroke=INK)
        fig.text(x0 - 56, 72, "Frame 1", anchor="start", weight="bold")
        moves = {8: -0.5, 12: 0.5, 18: 1.5}
        factors = {4: 0, 6: 1, 9: 2, 10: 3}
        n = draw_frame(
            fig, moves, factors, 150, 230, d, x0, rings={8, 12}, hidden={18, 6} if kept_claim else (),
            styles={e: "faintdashed" for e in ((18, 9), (12, 6), (18, 6))} if kept_claim else None,
        )
        if kept_claim:
            fig.edge(n[18], n[6], "claimout")
            fig.edge(n[8], n[4], "thick")
            fig.edge(n[12], n[4], "thick")
            fig.text(x0 + 110, 262, "8 and 12 both need 4: unplayable", size=14, weight="bold")
        else:
            for k, (m, f) in enumerate([(8, 4), (12, 6), (18, 9)], 1):
                fig.edge(n[m], n[f], "thick")
                fig.tag((n[m].x + n[f].x) / 2, (n[m].y + n[f].y) / 2, k, shape="square")
            fig.text(x0 + 110, 262, "winnable: drops 1, 2, 3", size=14, weight="bold")
    fig.lines(
        390,
        330,
        [
            "Double outline: 8 and 12, promoted into frame 1 from frame 2.  In frame 1, 8 lists 4 and 12 lists 6, 4.",
            "The claim (18, 6), light dashed pair, takes 18 and 6 out first, which leaves 8 and 12 one factor, 4.",
            "The promotion set {8, 12} would be lost, though 8 and 12 with the rest of the set are playable.",
        ],
        size=12,
        fill=MID,
    )
    fig.save()


FIGURES = [cycle_21, graphic_21, payment_graph_21, sources_21, frames_21, promotion_21, frame_matroids_21, cycle_98,
           parity_10, game_28, state_28, exchange_2873, cores_9170, gf3_45, extended_24]


def main():
    for figure in FIGURES:
        figure()


if __name__ == "__main__":
    main()
