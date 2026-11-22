// The cycle that an odd number with two prime factors closes (the semiprime formula, Theorem J3,
// docs/theory/arrival.md).
//
// G_n is the graph of the bottom-layers theorem: its vertices are the primes up to n/2 and one more vertex, g; each
// number of layer 2 is an edge, pq the edge p to q and p² the edge p to g.  U*₂(n) is the part of U*(n) in layer 2, a
// forest in G_n.  An odd n with two prime factors closes a cycle with U*₂(n − 1) when its two ends are already joined
// by a path of that forest; the cycle is that path and n, and x, the smallest member of the path, is the one that
// gives way: opt(n) = opt(n − 1) + n − x.
//
// Everything here comes from js/model.js: the forest is what descendingGreedy(n − 1) keeps in layer 2.  The figure of
// game 39's cycle (figures/semiprime-39.js) reads it, and tools/test.mjs checks it against the record.

import * as M from './model.js';

/** The ground vertex g, the other end of the edge p² of a square. */
export const GROUND = 'g';

/** The two ends of a number of layer 2 in G_n: its primes, the square p² with g. */
export function ends(m) {
  const [p, q] = M.primeFactors(m);
  return [p, q ?? GROUND];
}

/** Whether n is odd and has two prime factors, counted with multiplicity, so 9, 25 and 49 count: n has a layer 2
 *  cycle to close. */
export const isSemiprimeOdd = (n) => n % 2 === 1 && M.omega(n) === 2;

/** U*₂(m): the numbers of layer 2 that descending greedy keeps in game m, ascending. */
export const layer2Moves = (m) => M.descendingGreedy(m).filter((s) => M.omega(s) === 2);

/** The path of edges that joins the vertices from and to along the edges given, as { edges, vertices } from `from`
 *  to `to`, or null when they are not joined. */
export function joinPath(edges, from, to) {
  const back = new Map([[from, null]]);
  const todo = [from];
  while (todo.length && !back.has(to)) {
    const v = todo.shift();
    for (const e of edges) {
      const [a, b] = ends(e);
      const w = a === v ? b : b === v ? a : null;
      if (w === null || back.has(w)) continue;
      back.set(w, [v, e]);
      todo.push(w);
    }
  }
  if (!back.has(to)) return null;
  const path = { edges: [], vertices: [to] };
  for (let v = to; back.get(v); v = back.get(v)[0]) {
    path.edges.unshift(back.get(v)[1]);
    path.vertices.unshift(back.get(v)[0]);
  }
  return path;
}

/** What odd n with two prime factors makes of the layer 2 moves of game n − 1: { n, moves, closes, cycle, x }.  moves
 *  is U*₂(n − 1); closes, whether n's two ends are already joined by them; cycle, the path of edges (moves) that joins
 *  them, ascending, and x its smallest member; order and vertices, the same path in order from one end of n to the
 *  other, as moves and as the vertices between them; all null when n closes no cycle. */
export function cycleFacts(n) {
  if (!isSemiprimeOdd(n)) throw new Error(`${n} is not odd with two prime factors`);
  const moves = layer2Moves(n - 1);
  const [a, b] = ends(n);
  const path = joinPath(moves, a, b);
  const cycle = path ? [...path.edges].sort((p, q) => p - q) : null;
  return {
    n, moves, closes: Boolean(path), cycle, x: cycle ? cycle[0] : null, order: path ? path.edges : null,
    vertices: path ? path.vertices : null,
  };
}
