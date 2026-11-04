// Along the way: the parity bound first misses at game 78.  The figure draws the odd core that the bound's heaviest set
// holds there: nine numbers on a ring of the nine factors they list, each number between the factors it can pay with,
// and the two that list three factors (30 and 42) joined across the ring to their third.  A flat figure; the two
// promotions of the set wear a candidate's double outline.
//
// Everything is computed here from js/model.js and data/small.json: U*, the candidates (figures/search-tree.js), and
// the heaviest set of candidates that passes the parity test with U* (figures/ladder.js), by a search that stops a
// branch once it fails the test.  That set's weight is ΣU* plus the frame bound at the root, which is at most the
// parity bound (the comparison theorem, B6) and equal to it on every board measured.  Peel's residual of the set is
// the core drawn; it is checked to pass the test, and to be a core, each factor listed twice or more.  Only the order
// of the ring is written by hand, and it is checked against the core.

import * as M from '../js/model.js';
import { scene } from '../js/scene.js';
import { candidatesOf } from './search-tree.js';
import { listsOf, passesParityTest } from './ladder.js';

export const N = 78;
// The ring, by hand: each factor, then the number between it and the next.  30 and 42 also reach across the ring.
const RING = [6, 78, 26, 52, 4, 28, 14, 42, 21, 63, 9, 45, 15, 75, 25, 50, 10, 30];
const W = 380;
const H = 290;
const C = { x: 190, y: 143, a: 168, b: 120 };

const sum = (xs) => xs.reduce((a, b) => a + b, 0);
const sorted = (xs) => [...xs].sort((a, b) => a - b);

/** The heaviest set of game n's candidates that passes the parity test together with U*, and what it is worth. */
export function heaviestPassing(n) {
  const uStar = M.descendingGreedy(n);
  const cands = candidatesOf(n, uStar); // largest first
  const rest = cands.map((_, i) => sum(cands.slice(i)));
  let best = { weight: -1, set: [] };
  const walk = (i, chosen, weight) => {
    if (weight > best.weight) best = { weight, set: [...chosen] };
    if (i === cands.length || weight + rest[i] <= best.weight) return;
    const next = [...chosen, cands[i]];
    if (passesParityTest([...uStar, ...next])) walk(i + 1, next, weight + cands[i]);
    walk(i + 1, chosen, weight);
  };
  walk(0, [], 0);
  return { uStar, cands, promotions: sorted(best.set), bound: sum(uStar) + best.weight };
}

/** The facts the aside states, for game n: the bound, opt(n), the exchange, and the core. */
export function parityMiss(small, n = N) {
  const game = small.find((g) => g.n === n);
  const { uStar, promotions, bound } = heaviestPassing(n);
  const set = [...uStar, ...promotions];
  const { playable, stuck } = M.peel(set);
  if (playable || bound <= game.score) throw new Error(`game ${n}: the heaviest set that passes the test should not play`);
  const lists = listsOf(set);
  const core = stuck.map((x) => [x, lists.get(x)]);
  const listed = new Map();
  for (const [, list] of core) for (const f of list) listed.set(f, (listed.get(f) ?? 0) + 1);
  if (![...listed.values()].every((k) => k >= 2) || !core.every(([, list]) => list.length >= 2)) {
    throw new Error(`game ${n}: peel's residual should be a core`);
  }
  // The core passes the test, since its lists are part of the set's, so it is an odd core.
  const inn = promotions.filter((x) => !game.promotions.includes(x));
  const out = game.promotions.filter((x) => !promotions.includes(x));
  return {
    n, bound, opt: game.score, promotions, optimal: sorted(game.promotions), inn, out, core, factors: sorted(listed.keys()),
    threes: core.filter(([, list]) => list.length === 3).map(([x]) => x),
  };
}

/** Mount the figure in host, which holds an <svg>. */
export function mountExact78(host, { small }) {
  const f = parityMiss(small);
  const lists = new Map(f.core);
  const onRing = new Set(RING);
  if (RING.length !== f.core.length + f.factors.length || ![...lists.keys(), ...f.factors].every((x) => onRing.has(x))) {
    throw new Error(`game ${N}: the ring is not the core`);
  }
  RING.forEach((x, i) => {
    const sides = [RING[(i + RING.length - 1) % RING.length], RING[(i + 1) % RING.length]];
    if (lists.has(x) && !sides.every((g) => lists.get(x).includes(g))) throw new Error(`game ${N}: ${x} is not between its factors`);
  });
  const s = scene(host.querySelector('svg'), {
    width: W, height: H, className: 'is-dense exact-78',
    label: `The odd core of the set that the parity bound allows in game ${N}: the numbers ${f.core.map(([x]) => x).join(', ')}, which list the factors ${f.factors.join(', ')}.`,
  });
  const look = {};
  RING.forEach((x, i) => {
    const angle = -Math.PI / 2 + (2 * Math.PI * i) / RING.length;
    s.number(x, x);
    const isMember = lists.has(x);
    look[x] = {
      x: C.x + C.a * Math.cos(angle), y: C.y + C.b * Math.sin(angle), shape: isMember ? 1 : 0,
      ring: f.promotions.includes(x) ? 1 : 0,
    };
  });
  for (const [x, list] of lists) {
    for (const g of list) {
      s.line(`${x}-${g}`, x, g, 'thin');
      look[`${x}-${g}`] = {};
    }
  }
  s.states([look]).show(0);
  return { facts: f, highlight: (n, on) => s.highlight(n, on), goState: () => {} };
}
