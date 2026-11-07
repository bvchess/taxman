// The search of game 45, node by node: a branch and bound over its promotions, run in the page.  The tree grows from
// the root at the bottom, each node a chip with its bound, each branch labeled with the candidate it promotes ("21")
// or goes on without ("no 21"), the leaves in the order the search visits them, left to right.  Above the tree a
// strip of the candidates shows, for the node the search is at, which are promoted, which are still open, and which
// are out, with the best game found so far.  The time slider visits the nodes in the order the search does; a node
// whose bound cannot beat the best game found is drawn dashed and labeled with why it was dropped, and the leaf that
// gives the best game is filled.  Pointing at a node, or tapping it, tells what the search knows there.  Nothing is
// placed until the search reaches it, and nothing moves once placed.
//
// The facts are js/model.js's: U* is descending greedy; the candidates are the factors of the frames with freedoms
// that can be promoted alone, U* with them playable (as figures/game21.js finds them; their count is games.json's
// candidates); and a set of candidates can be promoted together exactly when U* with it is playable (the exactness
// theorem, X7).  The bound is the figure's own and simpler than the solver's: the sum of every candidate that could
// still be promoted alone with the node's promotions.  The search's answer is checked against the recorded game.
//
// URL parameter: tree=k opens state k (1, the root, to the last, the search done).

import * as M from '../js/model.js';
import { tooltip } from '../js/chart.js';
import { el, fmt, listWords } from '../js/vocab.js';
import { scene, timeline } from '../js/scene.js';

export const N = 45;
const COL = 64; // from one leaf to the next
const ROW = 60; // from one depth to the next
const PAD_X = 46; // from the first leaf's center to the figure's edge
const TOP = 36; // from the top of the figure to the highest leaf's center, with its label above
const BOTTOM = 22; // from the root's center to the bottom of the figure
const CHIP = { w: 46, h: 26 };
const LABEL_RISE = 24; // from a leaf's center to its label
const EDGE_GAP = 7; // from the middle of an edge to its label
const CHAR = 7.4; // the width of a character of figure text at its largest size, for the check that all fits
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
    const promoted = listWords([...bestSet].sort((a, b) => b - a));
    return `Every branch has been searched or dropped, so no set of promotions adds more than ${best}. ` +
      `Game ${t.n}'s optimal game promotes ${promoted}, and scores ${fmt(base)} + ${best} = ${fmt(score)}.`;
  }
  const node = nodes[k];
  if (!node.parent) {
    return `The search starts at the root, at the bottom: nothing is promoted yet, and all ${cands.length} ` +
      `candidates are open. A node's number is the most its branch can add to the ${fmt(base)} points of U*, the ` +
      `sum of its promoted and open candidates: at the root, all ${cands.length} together, ${node.bound}.`;
  }
  const { x, promote } = node.edge;
  const parts = [];
  if (promote) parts.push(`Promote ${x}.`);
  else {
    const back = node.parent.parent ? `Back at ${node.parent.bound}` : 'Back at the root';
    parts.push(`${back}, go on without ${x}, ${withLeft(node.live)}.`);
  }
  if (node.dropped.length) {
    const withP = node.P.length === 1 ? 'it' : listWords(node.P);
    parts.push(`With ${withP}, ${listWords(node.dropped)} can no longer be promoted, so ${leftWords(node.live)}.`);
  }
  const terms = [...node.P, ...node.live];
  if (node.kind === 'cut') {
    const than = node.bound < node.bestBefore ? 'less than' : 'no more than';
    parts.push(`This branch adds at most ${written(terms, node.bound)}, ${than} the best so far, ${node.bestBefore}, ` +
      'so the search drops it without looking further.');
  } else if (node.kind === 'best') {
    parts.push(`Promoting ${listWords(node.P)} adds ${written(node.P, node.bound)}: the best game so far.`);
  } else if (node.bestBefore > 0) {
    parts.push(`The bound is ${written(terms, node.bound)}, above the best so far, ${node.bestBefore}, ` +
      'so the search goes on.');
  } else {
    parts.push(`The bound falls to ${written(terms, node.bound)}.`);
  }
  return parts.join(' ');
}

// A node's chip: a small rounded box with its bound, which a look draws.  halo: the node the search is at; dashed: a
// dropped branch; kept: the leaf of the best game.
function chip(node) {
  const g = el('g', { class: 'st-chip' });
  const halo = el('rect', {
    x: -CHIP.w / 2 - 3, y: -CHIP.h / 2 - 3, width: CHIP.w + 6, height: CHIP.h + 6, rx: 9, fill: 'none',
    class: 'st-halo',
  }, g);
  const body = el('rect', {
    x: -CHIP.w / 2, y: -CHIP.h / 2, width: CHIP.w, height: CHIP.h, rx: 6, class: 'st-body',
  }, g);
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

// A chip drawn in HTML, as a sample for the key.
function sampleChip(bound, kind) {
  const c = document.createElement('span');
  c.className = `st-sample is-${kind}`;
  c.textContent = String(bound);
  return c;
}

// The candidates passed over on the way to a node: the x of every "no x" branch from the root.
function passedOver(node) {
  const out = [];
  for (let n = node; n.parent; n = n.parent) if (!n.edge.promote) out.push(n.edge.x);
  return out;
}

// What the search knows at a node, as the rows of a tooltip.
function tipRows(t, node) {
  const { cands } = t;
  const passed = passedOver(node);
  const ruledOut = cands.filter((x) => !node.P.includes(x) && !passed.includes(x) && !node.live.includes(x));
  const list = (xs) => (xs.length ? xs.join(', ') : 'none');
  const rows = [!node.parent ? 'The root' : node.edge.promote ? `Promote ${node.edge.x}` : `Without ${node.edge.x}`];
  rows.push(['promoted', list(node.P)]);
  if (passed.length) rows.push(['passed over', list(cands.filter((x) => passed.includes(x)))]);
  if (ruledOut.length) rows.push(['ruled out', list(ruledOut)]);
  rows.push(['open', list(node.live)], ['at most', written([...node.P, ...node.live], node.bound)]);
  if (node.kind === 'branch') rows.push(`It splits on ${node.live[0]}: promote it, or go on without it.`);
  else if (node.kind === 'cut') rows.push(`Dropped: at most ${node.bound}, and the best so far is ${node.bestBefore}.`);
  else rows.push(`A game: promoting ${listWords(node.P)} adds ${node.bound}.`);
  return rows;
}

/** Mount the figure in host, which holds a .st-strip, an <svg>, a .st-key, .fig-timeline and .fig-caption.  Returns
 *  the figure's api and the facts the page's prose quotes. */
export function mountSearchTree(host, { small }) {
  const t = searchTree(N);
  const game = small.find((g) => g.n === N);
  if (sum(t.uStar) + t.best !== game.score || sum(t.bestSet) !== sum(game.promotions)) {
    throw new Error(`search tree: the search of game ${N} does not find the recorded score`);
  }
  const { nodes } = t;
  if (nodes.filter((node) => node.kind === 'best').length !== 1) {
    throw new Error('search tree: the words assume the search has one best game');
  }

  // Places: the leaves in the order the search reaches them, left to right; a node above the middle of its children;
  // the root at the bottom, each depth a row higher.
  let col = 0;
  const place = (node) => {
    if (node.children.length === 0) node.x = PAD_X + COL * col++;
    else {
      node.children.forEach(place);
      node.x = sum(node.children.map((c) => c.x)) / node.children.length;
    }
  };
  place(nodes[0]);
  const depth = Math.max(...nodes.map((node) => node.depth));
  const W = 2 * PAD_X + COL * (col - 1);
  const H = TOP + ROW * depth + BOTTOM;
  for (const node of nodes) node.y = TOP + ROW * (depth - node.depth);

  // Everything must fit: the chips with their halos, a leaf's label above it, an edge's label beside its middle.
  const leafLabel = (node) => (node.kind === 'cut' ? `${node.bound < node.bestBefore ? '<' : '='} ${node.bestBefore}`
    : 'best so far');
  const edgeLabel = (node) => (node.edge.promote ? String(node.edge.x) : `no ${node.edge.x}`);
  const fits = (left, right, top, bottom) => left >= 0 && right <= W && top >= 0 && bottom <= H;
  for (const node of nodes) {
    let ok = fits(node.x - CHIP.w / 2 - 3, node.x + CHIP.w / 2 + 3, node.y - CHIP.h / 2 - 3, node.y + CHIP.h / 2 + 3);
    if (node.children.length === 0) {
      const half = (CHAR * leafLabel(node).length) / 2;
      ok = ok && fits(node.x - half, node.x + half, node.y - LABEL_RISE - 7, node.y);
    }
    if (node.parent) {
      const mid = (node.x + node.parent.x) / 2;
      const width = CHAR * edgeLabel(node).length;
      const reach = EDGE_GAP + width;
      ok = ok && (node.edge.promote ? fits(mid - reach, mid, 0, 0) : fits(mid, mid + reach, 0, 0));
    }
    if (!ok) throw new Error(`search tree: node ${node.id} does not fit the figure`);
  }

  const s = scene(host.querySelector(':scope > svg'), {
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
    const from = { x: node.parent.x, y: node.parent.y - CHIP.h / 2 }; // the parent's top, the child's bottom
    s.line(`e${node.id}`, from, { x: node.x, y: node.y + CHIP.h / 2 }, 'branch');
    s.text(`l${node.id}`, edgeLabel(node), { anchor: node.edge.promote ? 'end' : 'start' });
    s.node(`l${node.id}`).dataset.n = String(node.edge.x);
    if (node.kind === 'cut') s.text(`r${node.id}`, leafLabel(node), { anchor: 'middle' });
    if (node.kind === 'best') s.text(`r${node.id}`, (pos, look) => look.words ?? '', { anchor: 'middle' });
  }

  const last = nodes.length;
  const states = [];
  for (let k = 0; k <= last; k++) {
    const v = {};
    for (const node of nodes) {
      const i = node.id;
      if (i > k) {
        // The branch to the next node is drawn from its parent as the search arrives there.
        if (i === k + 1 && node.parent) v[`e${i}`] = { draw: 0 };
        continue;
      }
      const isNew = i === k;
      v[`n${i}`] = {
        x: node.x, y: node.y, halo: isNew ? 1 : 0, dashed: node.kind === 'cut' ? 1 : 0,
        kept: node.kind === 'best' ? 1 : 0,
        ...(isNew && node.parent ? { at: [0.45, 1] } : {}),
      };
      if (!node.parent) continue;
      v[`e${i}`] = isNew ? { at: [0, 0.6] } : {};
      const mid = { x: (node.x + node.parent.x) / 2, y: (node.y + node.parent.y) / 2 };
      const side = node.edge.promote ? -EDGE_GAP : EDGE_GAP;
      v[`l${i}`] = { x: mid.x + side, y: mid.y, ...(isNew ? { at: [0.3, 0.8] } : {}) };
      if (node.kind === 'cut' || node.kind === 'best') {
        v[`r${i}`] = {
          x: node.x, y: node.y - LABEL_RISE, halo: node.kind === 'best' ? 1 : 0, ...(isNew ? { at: [0.6, 1] } : {}),
          ...(node.kind === 'best' ? { words: k === last ? 'best game' : 'best so far' } : {}),
        };
      }
    }
    states.push(v);
  }
  s.states(states);

  // The strip of candidates: for the node the search is at, promoted, open, or out.
  const strip = host.querySelector('.st-strip');
  strip.setAttribute('aria-hidden', 'true');
  const boxes = new Map(t.cands.map((x) => {
    const box = document.createElement('span');
    box.className = 'st-cand';
    box.textContent = String(x);
    return [x, box];
  }));
  const bestText = document.createElement('span');
  bestText.className = 'st-best';
  const boxRow = document.createElement('span');
  boxRow.className = 'st-cands';
  boxRow.append(...boxes.values());
  const stripLabel = document.createElement('span');
  stripLabel.className = 'st-strip-label';
  stripLabel.textContent = 'candidates';
  strip.append(stripLabel, boxRow, bestText);
  const showStrip = (k) => {
    const node = nodes[k]; // none in the last state, when the search is done
    const promoted = node ? node.P : t.bestSet;
    const open = node ? node.live : [];
    for (const [x, box] of boxes) {
      box.dataset.state = promoted.includes(x) ? 'promoted' : open.includes(x) ? 'open' : 'out';
    }
    const bestNow = node ? (node.kind === 'best' ? node.bound : node.bestBefore) : t.best;
    const value = Object.assign(document.createElement('b'), { textContent: String(bestNow) });
    bestText.replaceChildren(k === last ? 'best: ' : 'best so far: ', value);
  };

  // The key: a sample of each kind of chip, with the numbers of the search.
  const cut = nodes.find((node) => node.kind === 'cut');
  const best = nodes.find((node) => node.kind === 'best');
  const key = host.querySelector('.st-key');
  for (const [sample, words] of [
    [sampleChip(nodes[0].bound, 'branch'), 'a node: the most its branch can add to U*'],
    [sampleChip(cut.bound, 'cut'), 'dropped: it can’t beat the best so far'],
    [sampleChip(best.bound, 'best'), 'a game: every candidate decided'],
  ]) {
    const item = document.createElement('li');
    item.append(sample, words);
    key.appendChild(item);
  }

  // The tooltip: what the search knows at a node, for the nodes the state shows.
  let state = 0;
  host.classList.add('st-host');
  const tip = tooltip(host);
  const showTip = (node) => {
    if (node.id > state) return;
    const at = chips.get(node.id).body.getBoundingClientRect();
    const over = host.getBoundingClientRect();
    tip.show(tipRows(t, node), at.left + at.width / 2 - over.left, at.top - over.top);
  };
  for (const node of nodes) {
    const g = chips.get(node.id).g;
    g.addEventListener('pointerenter', () => showTip(node));
    g.addEventListener('pointerdown', () => showTip(node));
    g.addEventListener('pointerleave', () => tip.hide());
  }
  document.addEventListener('pointerdown', (e) => {
    if (!e.target.closest?.('.st-chip')) tip.hide();
  });

  const firstBest = best.id;
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
      state = k;
      tip.hide();
      caption.textContent = words(t, k, game.score);
      host.dataset.state = String(k + 1);
      showStrip(k);
    },
    label: 'Search figure: step',
  });
  const labels = (n) => nodes.filter((node) => node.parent && node.edge.x === n).map((node) => s.node(`l${node.id}`));
  return {
    goState: (k) => s.go(Number(k) - 1),
    stopName: (k) => names[Number(k) - 1],
    highlight: (n, on) => {
      for (const label of labels(Number(n))) label.classList.toggle('is-named', on);
      boxes.get(Number(n))?.classList.toggle('is-named', on);
    },
    facts: {
      n: N, candidates: t.cands.length, sets: 2 ** t.cands.length, nodes: nodes.length,
      dropped: nodes.filter((node) => node.kind === 'cut').length, rootBound: nodes[0].bound, best: t.best,
      base: sum(t.uStar), score: game.score, promotions: [...t.bestSet].sort((a, b) => a - b),
    },
  };
}
