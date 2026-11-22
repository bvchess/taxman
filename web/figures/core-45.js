// The first odd core: game 45's {12, 18, 20, 30, 45}, the set of Proposition C3.  The figure draws its five numbers on
// a ring of the five factors they list, each number between two factors it can pay with, and 30, which lists three,
// joined across the ring to its third.  The five numbers are gray circles, in the set being tested, and the factors
// are squares.  It has the two views of the game 78 figure (figures/exact-78.js), the ring and the layers, drawn by
// figures/odd-core.js, which also writes the captions.
//
// The facts are computed from js/model.js and figures/ladder.js: the lists of each number, that the set passes the
// parity test, that peel is stuck on all five numbers, and that it is a core, each number listing at least two
// factors and each factor listed at least twice.  Only the set and the order of the ring are written by hand, and the
// ring is checked against the core.

import * as M from '../js/model.js';
import { mountOddCore } from './odd-core.js';
import { SETS, listsOf, passesParityTest } from './ladder.js';

export const N = 45;
// The set, once: the core of Proposition C3.
export const SET = [12, 18, 20, 30, 45];
// The ring, by hand: each factor, then the number between it and the next.  30 also reaches across to 6.
const RING = [10, 20, 4, 12, 6, 18, 9, 45, 15, 30];
// The ring view draws the core where the ladder figure, lower on the page, draws it (figures/ladder.js), so that the
// reader sees one shape twice.
const LADDER = SETS.find((spec) => spec.set.join() === [...SET].sort((x, y) => x - y).join());
if (!LADDER) throw new Error(`game ${N}: the ladder figure draws no set ${SET}`);
const VIEW_PARAM = 'core45-view';

/** The facts the figure states: the core's numbers with the factors each lists, the factors, and the numbers listing
 *  three.  Throws if the set is not an odd core. */
export function core45Facts() {
  if (!passesParityTest(SET)) throw new Error(`game ${N}: the set should pass the parity test`);
  const { playable, stuck } = M.peel(SET);
  if (playable || [...stuck].sort((a, b) => a - b).join() !== [...SET].sort((a, b) => a - b).join()) {
    throw new Error(`game ${N}: peel should be stuck on all of the set`);
  }
  const lists = listsOf(SET);
  const listed = new Map();
  for (const list of lists.values()) for (const f of list) listed.set(f, (listed.get(f) ?? 0) + 1);
  if (![...lists.values()].every((list) => list.length >= 2) || ![...listed.values()].every((k) => k >= 2)) {
    throw new Error(`game ${N}: the set should be a core`);
  }
  return {
    n: N,
    core: SET.map((x) => [x, lists.get(x)]),
    factors: [...listed.keys()].sort((a, b) => a - b),
    threes: SET.filter((x) => lists.get(x).length === 3),
  };
}

/** Mount the figure in host, which holds an <svg>. */
export function mountCore45(host) {
  const f = core45Facts();
  const label = `The odd core of game ${N}: the numbers ${f.core.map(([x]) => x).join(', ')}, which list the factors `
    + `${f.factors.join(', ')}.`;
  const { s } = mountOddCore(host, {
    n: N, ring: RING, lists: new Map(f.core), viewParam: VIEW_PARAM,
    size: { width: LADDER.w, height: LADDER.h }, at: LADDER.at, label,
  });
  return { facts: f, highlight: (n, on) => s.highlight(n, on), goState: () => {} };
}
