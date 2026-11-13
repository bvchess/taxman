// Along the way: the greedy that shouldn't work.  Six sources of game 2873, built from the primes 11, 13 and 17, have
// sets that can take no more of the six at two sizes, four and five, so a greedy pass could stop at four.  A flat
// figure in three states: the six with their maximal factors; the four that greedy keeps in an order that traps it;
// the five it keeps from the largest down.  The six are the theory's (docs/theory/sources.md, "Where exchange
// breaks"); everything said of them is computed here with js/model.js: which sets can be played, the payments, the
// orders that trap greedy and the sets that can take no more.
//
// The same six can be seen in their layers instead (figures/layer-view.js): the six sources, all of layer 3, in a
// column at the left, the five maximal factors, all of layer 2, in a column at the right, and a line from each source
// to each of its factors.  A switch above the figure moves every number and line from one layout to the other.
//
// URL parameters: trap=k opens state k, from 1; trap-view=layers opens the figure in layers.

import * as M from '../js/model.js';
import { SHAPE, fmt, listWords } from '../js/vocab.js';
import { scene, timeline } from '../js/scene.js';
import { holdHeight } from './hold-height.js';
import { layerColumns, mountViewSwitch, viewFromUrl } from './layer-view.js';

const N = 2873;
const SIX = [1573, 1859, 2057, 2197, 2431, 2873];
const W = 430;
const H = 282;
const SCALE = 1.2;
// The factors at the corners of a pentagon, in the order that lets each source with two of them sit on a side; 2431,
// with three, sits inside, and 2197, with one, outside.
const FACTOR_AT = { 143: [205, 40], 169: [300, 104], 221: [262, 214], 187: [148, 214], 121: [110, 104] };
const SOURCE_AT = { 2431: [205, 152], 2197: [392, 62] };
const SCORE_Y = 262;
const VIEW_PARAM = 'trap-view';
// The layers view: the pitch of the sources in their column, and the room under the columns for the score.
const PITCH = 46;
const FOOTER = 36;

const sum = (xs) => xs.reduce((a, b) => a + b, 0);
const COUNT = { 4: 'four', 5: 'five' };

// Descending greedy over the six in the order given: keep each that is still playable with those kept.
function greedyIn(order) {
  const kept = [];
  for (const s of order) if (M.isPlayable([...kept, s])) kept.push(s);
  return kept;
}

function permutations(xs) {
  if (xs.length <= 1) return [xs];
  return xs.flatMap((x, i) => permutations([...xs.slice(0, i), ...xs.slice(i + 1)]).map((rest) => [x, ...rest]));
}

/** The facts of the six: greedy's set from the largest down, a trap order and its set, the orders that trap greedy,
 *  and the sets of the six that can be played and can take no more of them. */
export function trapFacts() {
  for (const s of SIX) if (s <= N / 2 || s > N) throw new Error(`game ${N}: ${s} is not a source`);
  const down = [...SIX].sort((a, b) => b - a);
  const best = greedyIn(down);
  const orders = permutations(SIX);
  const results = orders.map((order) => ({ order, kept: greedyIn(order) }));
  const traps = results.filter((r) => r.kept.length < best.length);
  // Every trap order ends with the same two, and the page says which.
  const lastTwo = [...new Set(traps.map((r) => [...r.order.slice(-2)].sort((a, b) => a - b).join(',')))];
  if (lastTwo.length !== 1) throw new Error(`game ${N}: the trap orders do not all end with the same two sources`);
  const trapLast = lastTwo[0].split(',').map(Number);
  const trapsByLast = orders.filter((o) => trapLast.every((x) => o.slice(-2).includes(x))).length;
  if (trapsByLast !== traps.length) throw new Error(`game ${N}: not every order ending with ${trapLast} traps greedy`);
  const trap = traps.find((r) => r.order[0] === Math.max(...SIX)) ?? traps[0];
  if (trap.kept.some((x, i) => x !== trap.order[i])) throw new Error(`game ${N}: the trap order does not keep its first members`);
  // The sets that can be played and can take none of the others.
  const subsets = [];
  for (let mask = 1; mask < 1 << SIX.length; mask++) subsets.push(SIX.filter((_, i) => mask & (1 << i)));
  const playable = subsets.filter((set) => M.isPlayable(set));
  const maximal = playable.filter((set) => SIX.every((x) => set.includes(x) || !M.isPlayable([...set, x])));
  const sizes = [...new Set(maximal.map((set) => set.length))].sort((a, b) => a - b);
  const weights = maximal.map(sum);
  if (sum(best) !== Math.max(...weights)) throw new Error(`game ${N}: greedy's set is not the heaviest of the six`);
  return {
    n: N, six: SIX, best: [...best].sort((a, b) => b - a), trap: { order: trap.order, kept: [...trap.kept].sort((a, b) => b - a) },
    orders: orders.length, traps: traps.length, trapLast, maximal: maximal.length, sizes,
    trapIsLightest: sum(trap.kept) === Math.min(...weights),
  };
}

/** Mount the figure in host, which holds an <svg>, .fig-timeline and .fig-caption. */
export function mountTrap(host) {
  const f = trapFacts();
  const factors = [...new Set(SIX.flatMap(M.maximalFactors))].sort((a, b) => a - b);
  for (const x of factors) if (!FACTOR_AT[x]) throw new Error(`game ${N}: no place for the factor ${x}`);
  const placeOf = (s) => {
    if (SOURCE_AT[s]) return SOURCE_AT[s];
    const [a, b] = M.maximalFactors(s).map((x) => FACTOR_AT[x]);
    return [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
  };
  const layers = layerColumns({
    sources: [...SIX].sort((a, b) => b - a), factors, factorsOf: M.maximalFactors, width: W, pitch: PITCH,
    footer: FOOTER, maxScale: SCALE,
  });
  const graphPlaces = { ...Object.fromEntries(factors.map((x) => [x, { x: FACTOR_AT[x][0], y: FACTOR_AT[x][1] }])),
    ...Object.fromEntries(SIX.map((src) => [src, { x: placeOf(src)[0], y: placeOf(src)[1] }])) };
  const boxes = { graph: { width: W, height: H }, layers: { width: W, height: layers.height } };
  const spots = {
    graph: { places: graphPlaces, scale: SCALE, scoreY: SCORE_Y },
    layers: { places: layers.places, scale: SCALE, scoreY: layers.height - 20 },
  };
  const view = viewFromUrl(VIEW_PARAM);
  const s = scene(host.querySelector('svg'), {
    ...boxes[view],
    label: 'Six sources of game 2,873 and their maximal factors, and the sets that descending greedy keeps in two '
      + 'orders. The text under the figure describes the current state.',
  });
  for (const src of SIX) for (const x of M.maximalFactors(src)) s.line(`e${src}-${x}`, src, x, 'thin');
  for (const src of SIX) for (const x of M.maximalFactors(src)) s.line(`p${src}-${x}`, src, x, 'pay');
  for (const x of factors) s.number(x, x);
  for (const src of SIX) s.number(src, src);
  s.text('score', (pos) => (Math.round(pos) === 1 ? `${f.trap.kept.length} kept: ${fmt(sum(f.trap.kept))}` : `${f.best.length} kept: ${fmt(sum(f.best))}`), { kind: 'score', anchor: 'middle' });

  const base = (spot) => {
    const st = {};
    for (const x of factors) st[x] = { ...spot.places[x], shape: SHAPE.square, scale: SCALE };
    for (const src of SIX) {
      st[src] = { ...spot.places[src], shape: SHAPE.circle, scale: SCALE };
      for (const fx of M.maximalFactors(src)) st[`e${src}-${fx}`] = {};
    }
    return st;
  };
  // A set kept: gray circles, each paid for by the factor a run of peel gives it; the others squares, since the
  // taxman takes them, with their lines faded.
  const keeping = (spot, kept) => {
    const st = base(spot);
    const run = M.peel(kept);
    for (const src of SIX) {
      if (kept.includes(src)) st[src].kept = 1;
      else {
        Object.assign(st[src], { shape: SHAPE.square, halo: 1 });
        for (const fx of M.maximalFactors(src)) st[`e${src}-${fx}`] = { faded: 1 };
      }
    }
    for (const step of run.steps) st[`p${step.member}-${step.factor}`] = {};
    st.score = { x: W / 2, y: spot.scoreY };
    return st;
  };
  const lists = Object.fromEntries(Object.entries(spots)
    .map(([name, spot]) => [name, [base(spot), keeping(spot, f.trap.kept), keeping(spot, f.best)]]));
  s.states(lists[view]);

  const threeFactors = listWords([...M.maximalFactors(2431)].sort((a, b) => a - b));
  const refusedIn = (kept) => SIX.filter((x) => !kept.includes(x)).sort((a, b) => b - a);
  const words = [
    `Six sources of game 2,873, each with lines to its maximal factors. Most have two; 2431 has three, ${threeFactors}, `
      + `and 2197 has one, ${M.maximalFactors(2197)[0]}.`,
    `In the order ${listWords(f.trap.order)}, greedy keeps the first ${COUNT[f.trap.kept.length]}, and then neither ${f.trapLast[0]} nor ${f.trapLast[1]} fits: ${COUNT[f.trap.kept.length]} sources, ${fmt(sum(f.trap.kept))}. Every order that puts those two last does the same.`,
    `From the largest down, greedy keeps ${listWords(f.best)} and refuses ${listWords(refusedIn(f.best))}: ${COUNT[f.best.length]} sources, ${fmt(sum(f.best))}, the heaviest playable set of the six.`,
  ];
  const caption = host.querySelector('.fig-caption');
  holdHeight(caption, words);
  timeline(s, host.querySelector('.fig-timeline'), {
    stops: [{ at: 0, name: 'the six' }, { at: 1, name: 'a trap' }, { at: 2, name: 'from the largest' }],
    param: 'trap',
    fromParam: (value) => Number(value) - 1,
    toParam: (k) => String(k + 1),
    describe: (k) => words[k],
    title: (k) => {
      caption.textContent = words[k];
    },
    label: 'Six sources of game 2,873: state',
  });
  mountViewSwitch(host, s, { view, lists, boxes, param: VIEW_PARAM });
  return {
    facts: f,
    stopName: (k) => ['the six', 'a trap', 'from the largest'][Number(k) - 1],
    goState: (k) => s.go(Number(k) - 1),
    highlight: (n, on) => s.highlight(n, on),
  };
}
