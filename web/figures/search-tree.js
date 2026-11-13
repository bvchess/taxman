// The search of game 45, node by node: a branch and bound over its promotions, run in the page.  The tree grows from
// the root at the bottom, each node a chip with its bound, each branch labeled with the candidate it promotes ("21")
// or goes on without ("no 21"), the leaves in the order the search visits them, left to right.  Above the tree a
// strip of the candidates shows, for the node the search is at, which are promoted, which remain, and which
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
import { el, fmt, listWords, numberSVG, restyle } from '../js/vocab.js';
import { scene, timeline } from '../js/scene.js';

export const N = 45;
const COL = 76; // from one leaf to the next, at least
const ROW = 60; // from one depth to the next
const PAD_X = 48; // from the first leaf's center to the figure's edge
const TOP = 58; // from the top of the figure to the highest leaf's center, with its label above
const BOTTOM = 40; // from the root's center to the bottom of the figure, with its label below
const ROOT_DROP = 27; // from the root's center to the middle of its label
const ROOT = 'root'; // the root's label
const CHIP = { w: 62, h: 26 }; // wide enough for "21, 18" at the phone's font size
const LABEL_RISE = 25; // from a leaf's center to the lower line of its label
const LINE = 17; // from one line of a label to the next
const EDGE_GAP = 7; // from the middle of an edge to its label
const CHAR = 8.4; // the width of a character of an edge label at its largest size, for the check that all fits
const LABEL_GAP = 24; // between the labels of two leaves
// The widths of the leaf labels' lines, in the figure's units at the phone's font size (17px, bold for the best
// leaf), as measured in the page, and a digit's; each with a little margin.
const LINE_WIDTH = { bound: 53, 'best so far': 88, 'best game': 88 };
const DIGIT = 10.5;
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

// The sum written out: "21 + 18 + 8 = 47"; a single term is just its number.
function written(xs, total) {
  return xs.length > 1 ? `${xs.join(' + ')} = ${total}` : String(total);
}

function leftWords(live) {
  if (live.length === 0) return 'no candidate remains';
  return live.length === 1 ? `only ${live[0]} remains` : `${listWords(live)} remain`;
}

function withLeft(live) {
  if (live.length === 0) return 'with no candidate remaining';
  return live.length === 1 ? `with only ${live[0]} remaining` : `with ${listWords(live)} remaining`;
}

// The words under the figure, for state k.
function words(t, k, score) {
  const { nodes, cands, uStar, bestSet, best } = t;
  const base = sum(uStar);
  if (k === nodes.length) {
    const promoted = listWords([...bestSet].sort((a, b) => b - a));
    return `The search has explored or dropped every branch, so no set of promotions adds more than ${best}. ` +
      `Game ${t.n}'s optimal game promotes ${promoted}, and scores ${fmt(base)} + ${best} = ${fmt(score)}.`;
  }
  const node = nodes[k];
  if (!node.parent) {
    return `The search starts at the root, at the bottom, with nothing promoted and all ${cands.length} candidates ` +
      'remaining. Each node shows the candidates the search promoted on the way to it. Its bound is the most its branch can add ' +
      `to the ${fmt(base)} points of U*, the sum of its promoted and remaining candidates: at the root, all ` +
      `${cands.length} together, ${node.bound}.`;
  }
  const { x, promote } = node.edge;
  const parts = [];
  if (promote) parts.push(`Promote ${x}.`);
  else {
    const goOn = node.parent.parent ? `Back down where the search split on ${x}, go on without it`
      : `Back at the root, go on without ${x}`;
    parts.push(`${goOn}, ${withLeft(node.live)}.`);
  }
  if (node.dropped.length) {
    const withP = node.P.length === 1 ? 'it' : listWords(node.P);
    parts.push(`With ${withP}, the player can no longer promote ${listWords(node.dropped)}, so ${leftWords(node.live)}.`);
  }
  const terms = [...node.P, ...node.live];
  if (node.kind === 'cut') {
    const than = node.bound < node.bestBefore ? 'less than' : 'no more than';
    parts.push(`Its bound is ${written(terms, node.bound)}, ${than} the best so far, ${node.bestBefore}, so no game ` +
      'in this branch can beat it, and the search drops it without looking further.');
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

// The promotions of a node, largest first.
const promotedText = (node) => (node.P.length ? node.P.join(', ') : 'none');

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
  text.textContent = promotedText(node);
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

// A leaf's label: two short lines centered above the leaf, so that neighbors' labels never meet.  A look may give
// words, "best so far|39", to change what a label says.
function leafLabelMark(lines) {
  const text = el('text', { class: 'v-text v-text-label', 'text-anchor': 'middle' });
  const spans = lines.map(() => el('tspan', {}, text));
  const say = (parts) => parts.forEach((part, i) => { spans[i].textContent = part; });
  say(lines);
  return {
    node: text,
    draw(look) {
      if (look.words) say(look.words.split('|'));
      spans.forEach((span, i) => {
        span.setAttribute('x', String(look.x));
        span.setAttribute('y', String(look.y - LINE * (spans.length - 1 - i)));
      });
      text.style.fontWeight = Math.round(500 + 250 * look.halo);
      text.style.fill = look.halo > 0.5 ? 'var(--ink)' : 'var(--ink-2)';
      text.style.opacity = look.opacity;
      text.style.display = look.opacity < 0.002 ? 'none' : '';
    },
  };
}

// The candidates passed over on the way to a node: the x of every "no x" branch from the root.
function passedOver(node) {
  const out = [];
  for (let n = node; n.parent; n = n.parent) if (!n.edge.promote) out.push(n.edge.x);
  return out;
}

// What the search knows at a node, as the rows of a tooltip.
function tipRows(t, node, done) {
  const { cands } = t;
  const passed = passedOver(node);
  const ruledOut = cands.filter((x) => !node.P.includes(x) && !passed.includes(x) && !node.live.includes(x));
  const list = (xs) => (xs.length ? xs.join(', ') : 'none');
  if (node.live.length === 0 && node.P.length === 0) throw new Error('search tree: a game without promotions');
  const rows = [node.P.length ? `Promoted: ${list(node.P)}` : 'Nothing promoted'];
  if (passed.length) rows.push(['passed over', list(cands.filter((x) => passed.includes(x)))]);
  if (ruledOut.length) rows.push(['ruled out', list(ruledOut)]);
  rows.push(['remaining candidates', list(node.live)], ['bound', written([...node.P, ...node.live], node.bound)]);
  const than = node.bound < node.bestBefore ? 'less than' : 'no more than';
  const so = `the best so far, ${node.bestBefore}`;
  const game = `No candidate remains, so this is a game: promoting ${listWords(node.P)} adds ${node.bound}`;
  let note;
  if (node.kind === 'branch') {
    note = `This branch can add at most ${node.bound}, more than ${so}, so the search goes on. ` +
      `Next it decides ${node.live[0]}: promote it, or go on without it.`;
  } else if (node.kind === 'cut' && node.live.length) {
    note = `This branch can add at most ${node.bound}, ${than} ${so}, so the search drops it without looking further.`;
  } else if (node.kind === 'cut') note = `${game}, ${than} ${so}, so the search moves on.`;
  else note = `${game}, ${done ? 'and no other game adds more' : 'the best so far'}.`;
  rows.push({ note });
  return rows;
}

/** Mount the figure in host, which holds a .st-strip, an <svg>, .fig-timeline and .fig-caption.  Returns
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
  const leafLabel = (node) => [node.kind === 'cut' ? 'bound' : 'best so far', String(node.bound)];
  // Half the widest line a leaf's label ever has: the best leaf says "best so far", then "best game".
  const reach = (node) => Math.max(...leafLabel(node).map((line) => LINE_WIDTH[line] ?? DIGIT * line.length),
    node.kind === 'best' ? LINE_WIDTH['best game'] : 0) / 2;
  let leftEdge = PAD_X; // the last leaf's x, then the next
  let before = null;
  const place = (node) => {
    if (node.children.length === 0) {
      if (before) leftEdge += Math.max(COL, reach(before) + reach(node) + LABEL_GAP);
      node.x = leftEdge;
      before = node;
    } else {
      node.children.forEach(place);
      node.x = sum(node.children.map((c) => c.x)) / node.children.length;
    }
  };
  place(nodes[0]);
  const depth = Math.max(...nodes.map((node) => node.depth));
  const W = before.x + PAD_X;
  const H = TOP + ROW * depth + BOTTOM;
  for (const node of nodes) node.y = TOP + ROW * (depth - node.depth);

  // Everything must fit: the chips with their halos, a leaf's label above it, an edge's label beside its middle.
  const edgeLabel = (node) => (node.edge.promote ? String(node.edge.x) : `no ${node.edge.x}`);
  const fits = (left, right, top, bottom) => left >= 0 && right <= W && top >= 0 && bottom <= H;
  for (const node of nodes) {
    let ok = fits(node.x - CHIP.w / 2 - 3, node.x + CHIP.w / 2 + 3, node.y - CHIP.h / 2 - 3, node.y + CHIP.h / 2 + 3);
    if (node.children.length === 0) {
      ok = ok && fits(node.x - reach(node), node.x + reach(node), node.y - LABEL_RISE - LINE - 9, node.y);
    }
    if (node.parent) {
      const mid = (node.x + node.parent.x) / 2;
      const width = CHAR * edgeLabel(node).length;
      const reach = EDGE_GAP + width;
      ok = ok && (node.edge.promote ? fits(mid - reach, mid, 0, 0) : fits(mid, mid + reach, 0, 0));
    }
    if (!ok) throw new Error(`search tree: node ${node.id} does not fit the figure`);
  }
  const root = nodes[0];
  if (!fits(root.x - (DIGIT * ROOT.length) / 2, root.x + (DIGIT * ROOT.length) / 2, 0, root.y + ROOT_DROP + LINE / 2)) {
    throw new Error('search tree: the root\'s label does not fit the figure');
  }
  // Labels of leaves in one row must keep apart.
  const leaves = nodes.filter((node) => node.children.length === 0);
  for (const a of leaves) {
    for (const b of leaves) {
      if (a.id < b.id && a.y === b.y && Math.abs(a.x - b.x) < reach(a) + reach(b) + LABEL_GAP - 1e-6) {
        throw new Error(`search tree: the labels of leaves ${a.id} and ${b.id} meet`);
      }
    }
  }

  const s = scene(host.querySelector(':scope > svg'), {
    width: W, height: H, className: 'is-dense search-tree',
    label: `The search of game ${N}, node by node. The text under the figure describes the current step.`,
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
    if (node.kind === 'cut' || node.kind === 'best') {
      const label = leafLabelMark(leafLabel(node));
      s.custom(`r${node.id}`, label.node, (_, look) => label.draw(look), {
        defaults: { x: 0, y: 0, opacity: 1, halo: 0, words: '' },
        layer: 'text',
      });
    }
  }

  s.text('root', ROOT, { anchor: 'middle' });

  const last = nodes.length;
  const states = [];
  for (let k = 0; k <= last; k++) {
    const v = { root: { x: root.x, y: root.y + ROOT_DROP } };
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
          ...(node.kind === 'best' ? { words: `${k === last ? 'best game' : 'best so far'}|${node.bound}` } : {}),
        };
      }
    }
    states.push(v);
  }
  s.states(states);

  // The strip of candidates: for the node the search is at, promoted, remaining, or out.
  const strip = host.querySelector('.st-strip');
  strip.setAttribute('aria-hidden', 'true');
  const boxes = new Map(t.cands.map((x) => {
    const box = document.createElement('span');
    box.className = 'st-cand';
    box.append(numberSVG(x, { rendering: 'flat', looks: ['factor', 'candidate'], size: 36, pad: 5 }));
    return [x, box];
  }));
  // A remaining candidate is a square with a double outline, a promoted one a gray double circle, one the node has
  // decided not to promote a plain square; the halo names the one the text is on.
  const LOOKS = { promoted: ['move', 'kept', 'candidate'], remaining: ['factor', 'candidate'], out: ['factor'] };
  const paint = (box) => {
    const named = box.classList.contains('is-named') ? ['named'] : [];
    restyle(box.firstChild, ...LOOKS[box.dataset.state ?? 'remaining'], ...named);
  };
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
    const remaining = node ? node.live : [];
    for (const [x, box] of boxes) {
      box.dataset.state = promoted.includes(x) ? 'promoted' : remaining.includes(x) ? 'remaining' : 'out';
      paint(box);
    }
    const bestNow = node ? (node.kind === 'best' ? node.bound : node.bestBefore) : t.best;
    const value = Object.assign(document.createElement('b'), { textContent: String(bestNow) });
    bestText.replaceChildren(k === last ? 'best: ' : 'best so far: ', value);
  };

  const best = nodes.find((node) => node.kind === 'best');

  // The tooltip: what the search knows at a node, for the nodes the state shows.
  let state = 0;
  host.classList.add('st-host');
  const tip = tooltip(host);
  const showTip = (node) => {
    if (node.id > state) return;
    const at = chips.get(node.id).body.getBoundingClientRect();
    const over = host.getBoundingClientRect();
    tip.show(tipRows(t, node, state === last), at.left + at.width / 2 - over.left, at.top - over.top);
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
      const box = boxes.get(Number(n));
      if (box) {
        box.classList.toggle('is-named', on);
        paint(box);
      }
    },
    facts: {
      n: N, candidates: t.cands.length, sets: 2 ** t.cands.length, nodes: nodes.length,
      dropped: nodes.filter((node) => node.kind === 'cut').length, rootBound: nodes[0].bound, best: t.best,
      base: sum(t.uStar), score: game.score, promotions: [...t.bestSet].sort((a, b) => a - b),
    },
  };
}
