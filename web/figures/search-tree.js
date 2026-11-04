// The search of game 45, node by node: a branch and bound over its promotions, run in the page.  A flat figure: the
// tree grows from the root at the left, each node a chip with its bound, each branch labeled with the candidate it
// promotes ("21") or goes on without ("no 21").  The time slider visits the nodes in the order the search does; a
// node whose bound cannot beat the best game found is drawn dashed and marked "dropped", and the leaf that gives the
// best game is filled.  Nothing is placed until the search reaches it, and nothing moves once placed.
//
// The facts are js/model.js's: U* is descending greedy; the candidates are the factors of the frames with freedoms
// that can be promoted alone, U* with them playable (as figures/game21.js finds them; their count is games.json's
// candidates); and a set of candidates can be promoted together exactly when U* with it is playable (the exactness
// theorem, X7).  The bound is the figure's own and simpler than the solver's: the sum of every candidate that could
// still be promoted alone with the node's promotions.  The search's answer is checked against the recorded game.
//
// URL parameter: tree=k opens state k (1, the root, to the last, the search done).

import * as M from '../js/model.js';
import { el, fmt, listWords } from '../js/vocab.js';
import { scene, timeline } from '../js/scene.js';

export const N = 45;
const W = 468;
const H = 334;
const X0 = 30; // the root's center
const COL = 86; // from one depth to the next
const Y0 = 52; // the first leaf
const ROW = 42; // from one leaf to the next
const CHIP = { w: 44, h: 24 };
const sum = (xs) => xs.reduce((a, b) => a + b, 0);

/** The candidates of game n, largest first: the factors of the frames with freedoms that U* can take on alone. */
export function candidatesOf(n, uStar = M.descendingGreedy(n)) {
  return M.frameModel(n).filter((f) => !f.closed).flatMap((f) => f.factors)
    .filter((x) => M.isPlayable([...uStar, x])).sort((a, b) => b - a);
}

/** The search of game n, run: { uStar, cands, nodes, best, bestSet }.  Each node is { id, P, live, dropped, bound,
 *  parent, edge, depth, kind, bestBefore }, in the order the search visits them.  At a node it drops the candidates
 *  that can no longer join its promotions P; its bound is ΣP plus the sum of those left (live).  A node is 'cut' when
 *  its bound cannot beat the best game found, 'best' when no candidate is left and P beats it, and else it branches
 *  on its largest live candidate: first promoting it, then going on without it. */
export function searchTree(n) {
  const uStar = M.descendingGreedy(n);
  const cands = candidatesOf(n, uStar);
  const nodes = [];
  let best = 0; // the game with no promotions
  let bestSet = [];
  function visit(P, rest, parent, edge) {
    const live = rest.filter((x) => M.isPlayable([...uStar, ...P, x]));
    const node = {
      id: nodes.length, P, live, dropped: rest.filter((x) => !live.includes(x)), bound: sum(P) + sum(live), parent,
      edge, depth: parent ? parent.depth + 1 : 0, bestBefore: best, children: [],
    };
    nodes.push(node);
    parent?.children.push(node);
    if (node.bound <= best) node.kind = 'cut';
    else if (live.length === 0) {
      node.kind = 'best';
      best = sum(P);
      bestSet = P;
    } else {
      node.kind = 'branch';
      const [x, ...others] = live;
      visit([...P, x], others, node, { x, promote: true });
      visit(P, others, node, { x, promote: false });
    }
  }
  visit([], cands, null, null);
  return { n, uStar, cands, nodes, best, bestSet };
}

// The sum written out, when it is short: "21 + 18 + 8 = 47".
function written(xs, total) {
  return xs.length > 1 && xs.length <= 4 ? `${xs.join(' + ')} = ${total}` : String(total);
}

function leftWords(live) {
  if (live.length === 0) return 'no candidate is left';
  return live.length === 1 ? `only ${live[0]} is left` : `${listWords(live)} are left`;
}

function withLeft(live) {
  if (live.length === 0) return 'with no candidate left';
  return live.length === 1 ? `with only ${live[0]} left` : `with ${listWords(live)} left`;
}

// The words under the figure, for state k.
function words(t, k, score) {
  const { nodes, cands, uStar, bestSet, best } = t;
  const base = sum(uStar);
  if (k === nodes.length) {
    return `Every branch has been searched or dropped, so no set of promotions adds more than ${best}. Game ${t.n}'s optimal game promotes ${listWords([...bestSet].sort((a, b) => b - a))}, and scores ${fmt(base)} + ${best} = ${fmt(score)}.`;
  }
  const node = nodes[k];
  if (!node.parent) {
    return `The root: nothing is promoted yet, and all ${cands.length} candidates are open. Each number in the tree is what promotions add to the ${fmt(base)} points of U*. Together the candidates add ${node.bound}, so no game of ${t.n} adds more: the root's bound is ${node.bound}.`;
  }
  const { x, promote } = node.edge;
  const parts = [];
  if (promote) parts.push(`Promote ${x}.`);
  else parts.push(`${node.parent.parent ? 'Back up, and go' : 'Back at the root, go'} on without ${x}, ${withLeft(node.live)}.`);
  if (node.dropped.length) {
    parts.push(`With ${node.P.length === 1 ? 'it' : listWords(node.P)}, ${listWords(node.dropped)} can no longer be promoted, so ${leftWords(node.live)}.`);
  }
  const terms = [...node.P, ...node.live];
  if (node.kind === 'cut') {
    parts.push(`This branch adds at most ${written(terms, node.bound)}, ${node.bound < node.bestBefore ? 'less than' : 'no more than'} the best game's ${node.bestBefore}, so the search drops it.`);
  } else if (node.kind === 'best') {
    parts.push(`Promoting ${listWords(node.P)} adds ${written(node.P, node.bound)}: the best game so far.`);
  } else if (node.bestBefore > 0) {
    parts.push(`The bound is ${written(terms, node.bound)}, above the best game's ${node.bestBefore}, so the search goes on.`);
  } else {
    parts.push(`The bound falls to ${written(terms, node.bound)}.`);
  }
  return parts.join(' ');
}

// A node's chip: a small rounded box with its bound, which a look draws.  halo: the node the search is at; dashed: a
// dropped branch; kept: the leaf of the best game.
function chip(node) {
  const g = el('g', { class: 'st-chip' });
  const halo = el('rect', { x: -CHIP.w / 2 - 3, y: -CHIP.h / 2 - 3, width: CHIP.w + 6, height: CHIP.h + 6, rx: 9, fill: 'none', class: 'st-halo' }, g);
  const body = el('rect', { x: -CHIP.w / 2, y: -CHIP.h / 2, width: CHIP.w, height: CHIP.h, rx: 6, class: 'st-body' }, g);
  const text = el('text', { 'text-anchor': 'middle', dy: '0.35em', class: 'st-bound' }, g);
  text.textContent = String(node.bound);
  return { g, halo, body, text };
}

function drawChip(parts, look) {
  parts.g.setAttribute('transform', `translate(${look.x} ${look.y})`);
  parts.g.style.opacity = look.opacity;
  parts.g.style.display = look.opacity < 0.002 ? 'none' : '';
  parts.halo.style.strokeOpacity = 0.18 * look.halo;
  parts.body.style.strokeWidth = 1.5 + 0.9 * look.halo;
  parts.body.setAttribute('stroke-dasharray', look.dashed > 0.5 ? '4 3' : 'none');
  parts.body.style.fill = look.kept > 0.5 ? 'var(--kept)' : 'var(--paper)';
  parts.body.style.stroke = look.dashed > 0.5 ? 'var(--ink-3)' : 'var(--ink)';
  parts.text.style.fill = look.dashed > 0.5 ? 'var(--ink-3)' : 'var(--ink)';
  parts.text.style.fontWeight = look.kept > 0.5 || look.halo > 0.5 ? 750 : 600;
}

/** Mount the figure in host, which holds an <svg>, .fig-timeline and .fig-caption.  Returns the figure's api and the
 *  facts the page's prose quotes. */
export function mountSearchTree(host, { small }) {
  const t = searchTree(N);
  const game = small.find((g) => g.n === N);
  if (sum(t.uStar) + t.best !== game.score || sum(t.bestSet) !== sum(game.promotions)) {
    throw new Error(`search tree: the search of game ${N} does not find the recorded score`);
  }
  const { nodes } = t;

  // Places: the leaves in the order the search reaches them, top to bottom; a node level with the middle of its
  // children; each depth a column.
  let row = 0;
  const place = (node) => {
    node.x = X0 + node.depth * COL;
    if (node.children.length === 0) node.y = Y0 + ROW * row++;
    else {
      node.children.forEach(place);
      node.y = sum(node.children.map((c) => c.y)) / node.children.length;
    }
  };
  place(nodes[0]);
  if (Y0 + ROW * (row - 1) + CHIP.h / 2 + 14 > H) throw new Error('search tree: the leaves do not fit the figure');

  const s = scene(host.querySelector('svg'), {
    width: W, height: H, className: 'is-dense search-tree',
    label: `The search of game ${N}, node by node. The current step is described under the figure.`,
  });
  const chips = new Map();
  for (const node of nodes) {
    const parts = chip(node);
    chips.set(node.id, parts);
    s.custom(`n${node.id}`, parts.g, (_, look) => drawChip(parts, look), {
      defaults: { x: 0, y: 0, opacity: 1, halo: 0, dashed: 0, kept: 0 }, layer: 'number',
    });
    if (!node.parent) continue;
    const from = { x: node.parent.x + CHIP.w / 2, y: node.parent.y };
    const to = { x: node.x - CHIP.w / 2, y: node.y };
    s.line(`e${node.id}`, from, to, 'thin');
    s.text(`l${node.id}`, node.edge.promote ? String(node.edge.x) : `no ${node.edge.x}`, { anchor: 'end' });
    s.node(`l${node.id}`).dataset.n = String(node.edge.x);
    if (node.kind === 'cut') s.text(`d${node.id}`, 'dropped');
    if (node.kind === 'best') s.text(`d${node.id}`, 'best game');
  }
  s.text('best', (pos, look) => look.words ?? '', { kind: 'name', anchor: 'end' });

  const last = nodes.length;
  const states = [];
  for (let k = 0; k <= last; k++) {
    const v = {};
    const bestNow = k < last ? (nodes[k].kind === 'best' ? nodes[k].bound : nodes[k].bestBefore) : t.best;
    v.best = { x: W - 6, y: 16, words: k === last ? `best: ${bestNow}` : `best so far: ${bestNow}` };
    for (const node of nodes) {
      const i = node.id;
      if (i > k) {
        // The branch to the next node is drawn from its parent as the search arrives there.
        if (i === k + 1 && node.parent) v[`e${i}`] = { draw: 0 };
        continue;
      }
      const isNew = i === k;
      v[`n${i}`] = {
        x: node.x, y: node.y, halo: isNew ? 1 : 0, dashed: node.kind === 'cut' ? 1 : 0, kept: node.kind === 'best' ? 1 : 0,
        ...(isNew && node.parent ? { at: [0.45, 1] } : {}),
      };
      if (!node.parent) continue;
      v[`e${i}`] = isNew ? { bend: 1, at: [0, 0.6] } : { bend: 1 };
      const above = node.edge.promote ? -1 : 1;
      v[`l${i}`] = { x: node.x - CHIP.w / 2 - 5, y: node.y + above * 11, ...(isNew ? { at: [0.3, 0.8] } : {}) };
      if (node.kind === 'cut' || node.kind === 'best') {
        v[`d${i}`] = { x: node.x + CHIP.w / 2 + 6, y: node.y, halo: node.kind === 'best' ? 1 : 0, ...(isNew ? { at: [0.6, 1] } : {}) };
      }
    }
    states.push(v);
  }
  s.states(states);

  const firstBest = nodes.findIndex((node) => node.kind === 'best');
  const stops = [{ at: 0, name: 'the root' }, { at: firstBest, name: 'a first game' }, { at: last, name: 'done' }];
  const names = Array.from({ length: last + 1 }, (_, k) => stops.find((st) => st.at === k)?.name ?? `node ${k + 1}`);
  const caption = host.querySelector('.fig-caption');
  timeline(s, host.querySelector('.fig-timeline'), {
    stops,
    param: 'tree',
    fromParam: (value) => Number(value) - 1,
    toParam: (k) => String(k + 1),
    describe: (k) => words(t, k, game.score),
    title: (k) => {
      caption.textContent = words(t, k, game.score);
      host.dataset.state = String(k + 1);
    },
    label: 'Search figure: step',
  });
  const labels = (n) => nodes.filter((node) => node.parent && node.edge.x === n).map((node) => s.node(`l${node.id}`));
  return {
    goState: (k) => s.go(Number(k) - 1),
    stopName: (k) => names[Number(k) - 1],
    highlight: (n, on) => {
      for (const label of labels(Number(n))) label.classList.toggle('is-named', on);
    },
    facts: {
      candidates: t.cands.length, sets: 2 ** t.cands.length, nodes: nodes.length,
      dropped: nodes.filter((node) => node.kind === 'cut').length, rootBound: nodes[0].bound, best: t.best,
      base: sum(t.uStar), score: game.score, promotions: [...t.bestSet].sort((a, b) => a - b),
    },
  };
}
