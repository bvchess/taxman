// Game 39's cycle (Theorem J3, the semiprime formula), which is chapter 1's cycle again.  Of the moves of an optimal
// game of 38 the figure draws only those that 39 meets: 33, 22 and 26, the path of U*₂(38) between 3 and 13.  Each is
// a gray circle joined to its two primes, which are squares.  39 = 3 · 13 arrives joined to 3 and 13, and the four
// numbers make a cycle.  A stepped figure, as chapter 3's forest is, in three states: game 38's moves, 39 arriving,
// and the best choice, the exchange the theorem proves.  22, the smallest other member of the cycle, goes to the
// taxman, so it turns into an orange square (the key's taxman look), and 39 takes its place and turns gray.  The
// caption gives the score, opt(38) + 39 − 22, and what giving up 33 or 26 instead would score.
//
// Two views of the same marks, as chapter 1's cycle has (figures/layer-view.js): as a cycle, the four primes at the
// corners of a square and each number at the middle of its side, and in layers, the four numbers in a column at the
// left and the four primes in a column at the right, a thin line from each number to its two primes.  Switching keeps
// the state the timeline shows.
//
// The facts come from the model and the record, none typed: U*₂(38) from descendingGreedy (js/semiprime.js), the
// optimal set of game 38 and the scores from data/small.json and data/games.json.  The figure checks that U*₂(38) is
// the eight numbers the theory names, that 33, 22 and 26 are its path between 3 and 13, that each of the three sets
// (game 38's whole optimal set, less x, plus 39) is playable by the model's isPlayable, that each scores
// opt(38) + 39 − x, and that only giving up the smallest, 22, reaches opt(39).  Only the places of the cycle are
// written by hand.
//
// The caption holds the height of its longest text, so that stepping never moves the page.
//
// URL parameters: cycle39=k opens state k, from 1; cycle39-view=layers opens the figure in layers.

import * as M from '../js/model.js';
import { cycleFacts } from '../js/semiprime.js';
import { SIZE, fmt, listWords } from '../js/vocab.js';
import { scene, timeline } from '../js/scene.js';
import { holdHeight } from './hold-height.js';
import { CLEAR, layerColumns, mountViewSwitch, viewFromUrl } from './layer-view.js';

const N = 39;
const W = 330;
const H = 260;
// The cycle's places: the square 3, 11, 2, 13; each number stands at the middle of its side.
const CORNER = { 3: [70, 60], 11: [260, 60], 2: [260, 200], 13: [70, 200] };
const EXPECTED = [21, 22, 25, 26, 33, 34, 35, 38]; // U*₂(38), as the theory names it
const PATH = { order: [33, 22, 26], vertices: [3, 11, 2, 13] }; // its path from 3 to 13
const PITCH = 2 * SIZE.circle + CLEAR; // the layers' pitch, at full size
const VIEW_PARAM = 'cycle39-view';
const NAMES = { layers: 'in layers', graph: 'as a cycle' };

const sum = (list) => list.reduce((total, x) => total + x, 0);

/** The facts of the figure: U*₂(38), the cycle 39 closes and the number x that gives way, the optimal set of game 38,
 *  and for each of the cycle's members, in order along it, the set that results from giving it up and its score.
 *  small is data/small.json and games is data/games.json.  Throws if any fact is not as the theory says. */
export function semiprime39Facts(small, games) {
  const c = games.columns;
  const opt = { 38: c.score[37], 39: c.score[38] };
  const cf = cycleFacts(N);
  if (JSON.stringify(cf.moves) !== JSON.stringify(EXPECTED)) {
    throw new Error(`U*₂(38) is ${cf.moves}, not ${EXPECTED}`);
  }
  if (!cf.closes) throw new Error('39 closes no cycle with U*₂(38)');
  if (JSON.stringify([cf.order, cf.vertices]) !== JSON.stringify([PATH.order, PATH.vertices])) {
    throw new Error(`the path of U*₂(38) from 3 to 13 is ${cf.order}, not ${PATH.order}`);
  }
  const best = small.find((g) => g.n === 38).moves;
  const set38 = [...best].sort((a, b) => a - b);
  if (sum(set38) !== opt[38]) throw new Error(`the moves of game 38 sum to ${sum(set38)}, not ${opt[38]}`);
  if (JSON.stringify(set38.filter((m) => M.omega(m) === 2)) !== JSON.stringify(cf.moves)) {
    throw new Error('the optimal set of game 38 has other layer 2 moves than U*₂(38)');
  }
  const gives = cf.order.map((x) => {
    const set = [...set38.filter((m) => m !== x), N].sort((a, b) => a - b);
    if (!M.isPlayable(set)) throw new Error(`game 38's optimal set less ${x}, plus 39, is not playable`);
    const total = sum(set);
    if (total !== opt[38] + N - x) {
      throw new Error(`the set that gives up ${x} scores ${total}, not opt(38) + 39 − ${x}`);
    }
    return { x, set, score: total };
  });
  const reaching = gives.filter((g) => g.score === opt[39]).map((g) => g.x);
  if (reaching.length !== 1 || reaching[0] !== cf.x) throw new Error(`the sets that reach opt(39) give up ${reaching}`);
  if (!M.isPlayable(set38)) throw new Error('the optimal set of game 38 is not playable');
  return { n: N, opt, moves: cf.moves, cycle: cf.cycle, order: cf.order, vertices: cf.vertices, x: cf.x, set38, gives };
}

const primes = (m) => M.primeFactors(m);

function captions(f) {
  const others = f.gives.filter((g) => g.x !== f.x);
  const [from, to] = [f.vertices[0], f.vertices[f.vertices.length - 1]];
  return [
    `The figure draws only ${listWords(f.order)}, the moves of game 38 that join ${from} to ${to}. Game 38 plays `
      + 'others too. Each move is a gray circle joined to its two primes, which are squares.',
    `39 = ${primes(N).join(' · ')} arrives, joined to ${from} and ${to}.`,
    `39 takes the place of ${f.x}, the smallest member of the cycle, and the score is ${fmt(f.opt[38])} + 39 − ${f.x} `
      + `= ${fmt(f.opt[39])}, the best score of game 39. Giving up ${others.map((g) => g.x).join(' or ')} instead `
      + `would score ${others.map((g) => fmt(g.score)).join(' or ')}.`,
  ];
}

/** Mount the figure in host, which holds an <svg>, .fig-timeline and .fig-caption. */
export function mountSemiprime39(host, { small, games }) {
  const f = semiprime39Facts(small, games);
  const drawn = [...f.order, N].sort((a, b) => b - a); // the numbers of the cycle, the largest at the top
  const view = viewFromUrl(VIEW_PARAM);
  const layers = layerColumns({ sources: drawn, factors: f.vertices, factorsOf: primes, width: W, pitch: PITCH });
  const boxes = { graph: { width: W, height: H }, layers: { width: W, height: layers.height } };
  const label = 'The cycle that 39 closes: the numbers 33, 22, 26 and 39, each joined to its two primes among 3, 11, 2 '
    + 'and 13. The text under the figure describes the current state.';
  const s = scene(host.querySelector('svg'), { ...boxes[view], label });
  // Lines first, so that the numbers sit on them.
  for (const m of drawn) for (const p of primes(m)) s.line(`e${m}-${p}`, m, p, 'thin');
  for (const p of f.vertices) s.number(p, p);
  for (const m of drawn) s.number(m, m);

  const places = {
    graph: {
      ...Object.fromEntries(f.vertices.map((p) => [p, { x: CORNER[p][0], y: CORNER[p][1] }])),
      ...Object.fromEntries(drawn.map((m) => {
        const [a, b] = primes(m).map((p) => CORNER[p]);
        return [m, { x: (a[0] + b[0]) / 2, y: (a[1] + b[1]) / 2 }];
      })),
    },
    layers: layers.places,
  };
  const statesIn = (at) => {
    const base = () => {
      const st = {};
      for (const p of f.vertices) st[p] = { ...at[p], shape: 0 };
      for (const m of f.order) {
        st[m] = { ...at[m], shape: 1, kept: 1 };
        for (const p of primes(m)) st[`e${m}-${p}`] = {};
      }
      return st;
    };
    const arrived = () => {
      const st = base();
      st[N] = { ...at[N], shape: 1 };
      for (const p of primes(N)) st[`e${N}-${p}`] = {};
      return st;
    };
    // The exchange: x goes to the taxman, an orange square, and 39 takes its place in the set, a gray circle.
    const best = () => {
      const st = arrived();
      st[f.x] = { ...st[f.x], shape: 0, kept: 0, taxman: 1 };
      for (const p of primes(f.x)) st[`e${f.x}-${p}`] = { faded: 1 };
      st[N] = { ...st[N], kept: 1 };
      return st;
    };
    const list = [base(), arrived(), best()];
    list.forEach((st) => { st._duration = 650; });
    return list;
  };
  const lists = { graph: statesIn(places.graph), layers: statesIn(places.layers) };
  s.states(lists[view]);

  const words = captions(f);
  const caption = host.querySelector('.fig-caption');
  holdHeight(caption, words);

  const last = s.count - 1;
  const stopNames = ['game 38', '39 arrives', 'the best choice'];
  timeline(s, host.querySelector('.fig-timeline'), {
    stops: stopNames.map((name, at) => ({ at, name })),
    param: 'cycle39',
    fromParam: (value) => Number(value) - 1,
    toParam: (k) => String(k + 1),
    describe: (k) => words[k],
    title: (k) => {
      caption.textContent = words[k];
    },
    label: 'Game 39’s cycle: state',
  });
  // Switching view glides every mark to its place in the other view, at the state the timeline shows.
  mountViewSwitch(host, s, { view, lists, boxes, param: VIEW_PARAM, names: NAMES });
  return {
    facts: f,
    stopName: (k) => stopNames[Number(k) - 1],
    goState: (k) => s.go(Number(k) - 1),
    highlight: (n, on) => s.highlight(n, on),
  };
}
