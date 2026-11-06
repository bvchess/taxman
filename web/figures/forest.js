// Layer 2 of game 100 as a graph on the primes: each source with two prime factors is the edge between them, and
// descending greedy takes the edges from the largest down, refusing each one that would close a loop with those it
// kept.  It is the loop of chapter 1 at work.  A flat figure, stepped: the edges, one state for each source greedy
// decides, and the tree it leaves.
//
// What greedy keeps comes from js/model.js (descendingGreedy), and the figure checks it against the loop rule: every
// source it refuses closes a loop with the ones it kept before, which the figure finds and names, and no source it
// keeps does.  Only the places are written by hand: the primes 2, 3, 5 and 7 in a row, the larger primes in diamonds
// between them and in a fan to the left, each source at the middle of its edge.
//
// The same numbers can be seen in their layers instead: the sources in a column at the left, in the order greedy takes
// them, the primes in a column at the right, a thin line from each source to each of its two primes, as chapter 2
// draws the layers.  A switch above the figure moves every number and line from one layout to the other, and the
// timeline stays where it is.
//
// URL parameters: forest=k opens state k, from 1; forest-view=layers opens the figure in layers.

import * as M from '../js/model.js';
import { SHAPE, SIZE, listWords } from '../js/vocab.js';
import { scene, timeline } from '../js/scene.js';
import { holdHeight } from './hold-height.js';

const N = 100;
const W = 530;
const H = 316;
const HUB = 158;
const PRIME_AT = {
  2: [120, HUB], 3: [250, HUB], 5: [380, HUB], 7: [510, HUB],
  29: [185, HUB - 84], 31: [185, HUB + 84], 17: [315, HUB - 84], 19: [315, HUB + 84],
  11: [445, HUB - 84], 13: [445, HUB + 84],
  23: [250, HUB + 142],
  37: [20, HUB - 128], 41: [20, HUB - 44], 43: [20, HUB + 44], 47: [20, HUB + 128],
};

// The layers view: how far each line of numbers stands from the middle of the box, and the margin before the first
// number and after the last.
const LINE_GAP = 100;
const MARGIN = 24;
const VIEWS = {
  graph: 'as a graph',
  layers: 'in layers',
};

const VIEW_PARAM = 'forest-view';

const COUNT = { 3: 'three', 4: 'four', 5: 'five', 6: 'six', 7: 'seven', 8: 'eight' };

const product = (s) => {
  const [p, q] = M.primeFactors(s);
  return q === undefined ? `${p} · ${p}` : `${p} · ${q}`;
};

/** The facts of the figure: the primes up to n/2, the sources of layer 2 in greedy's order, and for each whether
 *  greedy keeps it and, if not, the loop it would close. */
export function forestFacts(n = N) {
  const primes = [];
  for (let p = 2; p <= n / 2; p++) if (M.omega(p) === 1) primes.push(p);
  const scan = M.sources(n).filter((s) => M.omega(s) === 2).reverse();
  const uStar = new Set(M.descendingGreedy(n));
  const ends = (s) => {
    const [p, q] = M.primeFactors(s);
    return [p, q ?? 'ground'];
  };
  const kept = [];
  // The path between two primes along the kept edges, as the edges it uses, or null.
  const path = (from, to) => {
    const back = new Map([[from, null]]);
    const todo = [from];
    while (todo.length) {
      const v = todo.shift();
      if (v === to) break;
      for (const e of kept) {
        const [a, b] = ends(e);
        const w = a === v ? b : b === v ? a : null;
        if (w === null || back.has(w)) continue;
        back.set(w, [v, e]);
        todo.push(w);
      }
    }
    if (!back.has(to)) return null;
    const edges = [];
    const nodes = [to];
    for (let v = to; back.get(v); v = back.get(v)[0]) {
      edges.push(back.get(v)[1]);
      nodes.push(back.get(v)[0]);
    }
    return { edges, nodes };
  };
  const steps = scan.map((s) => {
    const [p, q] = ends(s);
    const loop = path(p, q);
    const keep = uStar.has(s);
    if (keep === Boolean(loop)) {
      throw new Error(`layer 2 of game ${n}: greedy ${keep ? 'keeps' : 'refuses'} ${s}, which ${loop ? 'closes' : 'closes no'} loop`);
    }
    if (keep) kept.push(s);
    return { s, keep, loop: loop ? { edges: [...loop.edges].reverse(), primes: [...loop.nodes].reverse() } : null };
  });
  const refused = steps.filter((st) => !st.keep).map((st) => st.s);
  const reached = new Set(kept.flatMap(ends));
  return { n, primes, scan, steps, kept, refused, spans: primes.every((p) => reached.has(p)) };
}

// On a narrow screen the figure stands on end: in the graph every place is reflected across the diagonal, so the row of
// the primes 2, 3, 5 and 7 becomes a column, and the columns of the layers are longer, so their marks are larger.
const boxOf = (upright) => (upright ? { w: H, h: W } : { w: W, h: H });

/** The places of the graph: the primes where PRIME_AT puts them, each source at the middle of its edge. */
function graphPlaces(f, upright) {
  const at = ([x, y]) => (upright ? { x: y, y: x } : { x, y });
  const places = {};
  for (const p of f.primes) places[p] = at(PRIME_AT[p]);
  for (const s of f.scan) {
    const [a, b] = M.primeFactors(s).map((p) => places[p]);
    places[s] = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
  }
  return places;
}

/** The places of the layers, in the figure's two orientations as the graph's: the sources in the layer 2 line, in the
 *  order greedy takes them, and the primes in the layer 1 line, in the order of the mean place of their sources, so
 *  that few lines cross.  Upright the lines are columns, layer 1 at the right; in the landscape box they are rows,
 *  layer 1 along the bottom, the columns reflected across the diagonal.  The marks are as large as the pitch
 *  lets them be. */
function layerPlaces(f, upright) {
  const { w, h } = boxOf(upright);
  const length = upright ? h : w;
  const pitch = (count) => (length - 2 * MARGIN) / (count - 1);
  const scale = Math.min(1, (0.95 * pitch(f.scan.length)) / (2 * SIZE.circle));
  const meanPlace = (p) => {
    const mine = f.scan.map((s, i) => (M.primeFactors(s).includes(p) ? i : null)).filter((i) => i !== null);
    return mine.reduce((a, b) => a + b, 0) / mine.length;
  };
  const primes = [...f.primes].sort((a, b) => meanPlace(a) - meanPlace(b) || b - a);
  const places = {};
  const line = (list, side) => list.forEach((id, i) => {
    const along = MARGIN + i * pitch(list.length);
    const across = (upright ? w : h) / 2 + side * LINE_GAP;
    places[id] = { ...(upright ? { x: across, y: along } : { x: along, y: across }), scale };
  });
  line(f.scan, -1);
  line(primes, 1);
  return places;
}

const PLACES = { graph: graphPlaces, layers: layerPlaces };

/** Every state of the figure, laid out as view says. */
function states(f, upright, view) {
  const places = PLACES[view](f, upright);
  const base = () => {
    const st = {};
    for (const p of f.primes) st[p] = { ...places[p], shape: SHAPE.square };
    for (const s of f.scan) {
      st[s] = { ...places[s], shape: SHAPE.circle };
      for (const p of M.primeFactors(s)) st[`e${s}-${p}`] = {};
    }
    return st;
  };
  const list = [base()];
  f.steps.forEach((step, i) => {
    const st = base();
    // Every source decided so far, kept (gray) or refused (dashed, its edge out of focus).
    for (const done of f.steps.slice(0, i + 1)) {
      st[done.s] = { ...st[done.s], ...(done.keep ? { kept: 1 } : { dashed: 1, faded: done === step ? 0 : 0.6 }) };
      if (!done.keep && done !== step) for (const p of M.primeFactors(done.s)) st[`e${done.s}-${p}`] = { faded: 1 };
    }
    // The source greedy is deciding wears a halo, and so does the loop it would close.
    st[step.s].halo = 1;
    for (const p of M.primeFactors(step.s)) st[`e${step.s}-${p}`] = { halo: 1 };
    if (step.loop) {
      for (const e of step.loop.edges) {
        st[e].halo = 1;
        for (const p of M.primeFactors(e)) st[`e${e}-${p}`] = { halo: 1 };
      }
      for (const p of step.loop.primes) st[p].halo = 1;
    }
    st._duration = 650;
    list.push(st);
  });
  const tree = base();
  for (const step of f.steps) {
    if (step.keep) tree[step.s] = { ...tree[step.s], kept: 1 };
    else {
      tree[step.s] = { ...tree[step.s], dashed: 1, faded: 0.6, opacity: 0.7 };
      for (const p of M.primeFactors(step.s)) tree[`e${step.s}-${p}`] = { faded: 1, opacity: 0.6 };
    }
  }
  list.push(tree);
  return list;
}

function captions(f) {
  const words = [
    `Game ${f.n}'s ${f.scan.length} sources with two prime factors, each on the edge between its two primes. The ${f.primes.length} primes up to ${f.n / 2} are the squares.`,
  ];
  let keptSoFar = 0;
  for (const step of f.steps) {
    if (step.keep) {
      keptSoFar++;
      words.push(`${step.s} = ${product(step.s)} closes no loop with the edges kept, so greedy keeps it: ${keptSoFar} kept so far.`);
    } else {
      const k = step.loop.primes.length;
      words.push(`${step.s} = ${product(step.s)} would close a loop with ${listWords(step.loop.edges)}: ${COUNT[k] ?? k} numbers on ${COUNT[k] ?? k} primes, ${listWords(step.loop.primes)}, as in chapter 1's loop. Greedy refuses it.`);
    }
  }
  words.push(`Greedy keeps ${f.kept.length} of the ${f.scan.length} and refuses ${listWords(f.refused)}. What it keeps ${f.spans ? `joins all ${f.primes.length} primes with no loop: a tree` : 'has no loop: a forest'}.`);
  return words;
}

const VIEW_WORDS = {
  graph: 'as a graph, each source an edge between its two primes',
  layers: 'in their layers, each source joined to its two primes',
};
const labelOf = (view) => `Layer 2 of game ${N}: the primes up to ${N / 2} and the sources with two prime factors, `
  + `which descending greedy takes from the largest down, ${VIEW_WORDS[view]}. `
  + 'The current state is described under the figure.';

/** The switch between the views: a button for each, the one shown pressed. */
function viewSwitch(onPick) {
  const box = document.createElement('div');
  box.className = 'fig-view';
  box.setAttribute('role', 'group');
  box.setAttribute('aria-label', 'View of the figure');
  box.innerHTML = `<span class="fig-view-word">Show</span>${Object.entries(VIEWS)
    .map(([view, name]) => `<button type="button" class="btn" data-view="${view}">${name}</button>`).join('')}`;
  const buttons = [...box.querySelectorAll('button')];
  for (const b of buttons) b.addEventListener('click', () => onPick(b.dataset.view));
  const show = (view) => buttons.forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.view === view)));
  return { box, show };
}

/** Mount the figure in host, which holds an <svg>, .fig-timeline and .fig-caption. */
export function mountForest(host) {
  const f = forestFacts();
  for (const p of f.primes) if (!PRIME_AT[p]) throw new Error(`layer 2 of game ${N}: no place for the prime ${p}`);
  for (const s of f.scan) if (M.primeFactors(s).length !== 2) throw new Error(`layer 2 of game ${N}: ${s} is a square, with no edge between two primes`);
  const upright = host.clientWidth > 0 && host.clientWidth < 480;
  const lists = Object.fromEntries(Object.keys(VIEWS).map((view) => [view, states(f, upright, view)]));
  let view = new URLSearchParams(location.search).get(VIEW_PARAM) === 'layers' ? 'layers' : 'graph';
  const { w, h } = boxOf(upright);
  const s = scene(host.querySelector('svg'), { width: w, height: h, label: labelOf(view) });
  // Lines first, so that the numbers sit on them.
  for (const src of f.scan) for (const p of M.primeFactors(src)) s.line(`e${src}-${p}`, src, p, 'thin');
  for (const p of f.primes) s.number(p, p);
  for (const src of f.scan) s.number(src, src);
  s.states(lists[view]);
  const words = captions(f);
  const last = s.count - 1;
  const caption = host.querySelector('.fig-caption');
  holdHeight(caption, words);
  timeline(s, host.querySelector('.fig-timeline'), {
    stops: [
      { at: 0, name: 'the edges' },
      { at: 1, until: last - 1, name: `from ${f.scan[0]} down` },
      { at: last, name: 'the tree' },
    ],
    param: 'forest',
    fromParam: (value) => Number(value) - 1,
    toParam: (k) => String(k + 1),
    describe: (k) => words[k],
    title: (k) => {
      caption.textContent = words[k];
    },
    label: `Layer 2 of game ${N}: state`,
  });
  // Switching glides every mark to its place in the other layout, at the state the timeline shows.
  const switcher = viewSwitch((to) => {
    if (to === view) return;
    s.stop();
    const k = Math.round(s.pos);
    view = to;
    s.states(lists[view]);
    s.morph(Object.fromEntries(s.looksOf(k)));
    s.svg.setAttribute('aria-label', labelOf(view));
    switcher.show(view);
    const url = new URL(location.href);
    if (view === 'layers') url.searchParams.set(VIEW_PARAM, view);
    else url.searchParams.delete(VIEW_PARAM);
    history.replaceState(null, '', url);
  });
  switcher.show(view);
  host.insertBefore(switcher.box, s.svg);
  return {
    facts: f,
    /** The state, from 1, at which greedy decides source x. */
    stateOf: (x) => f.scan.indexOf(x) + 2,
    stopName: (k) => (Number(k) === 1 ? 'the edges' : Number(k) === s.count ? 'the tree' : `greedy at ${f.scan[Number(k) - 2]}`),
    goState: (k) => s.go(Number(k) - 1),
    highlight: (n, on) => s.highlight(n, on),
  };
}
