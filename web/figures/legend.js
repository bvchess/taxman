// The legend: every mark of the vocabulary, each drawn twice by vocab.js itself, flat as the explanatory figures draw
// it and raised as the reader's own pieces are, so the legend cannot drift from the figures.  The chapter shows the
// marks a reader needs; the style page shows every entry, with when to use it.

import {
  BOX, LINE_LOOK, NUMBER, TAG, boxMark, el, installDefs, lineMark, look, numberMark, setBox, setLine, setNumber, setTag,
  setToken, tagMark, token,
} from '../js/vocab.js';

const W = 60;
const H = 50;

function svgBox(w = W, h = H) {
  installDefs();
  return el('svg', { viewBox: `0 0 ${w} ${h}`, width: w * 0.8, height: h * 0.8, 'aria-hidden': 'true', class: 'v-figure' });
}

/** Draw the number label with the look parts at (x, y) in svg, in the rendering given; returns its full look. */
export function put(svg, rendering, label, x, y, ...parts) {
  const p = { ...NUMBER, ...look(...parts), x, y };
  if (rendering === 'raised') {
    const t = token(label);
    svg.appendChild(t);
    setToken(t, p);
  } else setNumber(numberMark(svg, label), p);
  return p;
}

function joined(svg, rendering, style, a, b) {
  const m = lineMark(svg, style, rendering);
  svg.insertBefore(m.g, svg.firstChild);
  setLine(m, a, b, { ...LINE_LOOK, sag: rendering === 'raised' && style !== 'pay' ? 6 : 0 });
}

const one = (label, ...parts) => (r) => {
  const svg = svgBox();
  put(svg, r, label, W / 2, H / 2 - 1, ...parts);
  return svg;
};

const pair = (style, [a, ...pa], [b, ...pb]) => (r) => {
  const svg = svgBox();
  const p = put(svg, r, a, 14, 15, ...pa, { scale: 0.72 });
  const q = put(svg, r, b, 46, 35, ...pb, { scale: 0.72 });
  joined(svg, r, style, p, q);
  return svg;
};

function frameSample(closed) {
  return (r) => {
    const svg = svgBox();
    setBox(boxMark(svg, r), { ...BOX, x: 6, y: 8, w: 48, h: 34, dashed: closed ? 1 : 0 });
    return svg;
  };
}

function promotionSample(r) {
  const svg = svgBox();
  if (r === 'raised') {
    // A tile turns over and lands as a coin.
    el('path', { d: 'M20 14 Q30 2 40 14', fill: 'none', stroke: 'var(--ink-2)', 'stroke-width': 1.5, 'stroke-linecap': 'round' }, svg);
    el('path', { d: 'm36.4 11.8 3.8 2.6.3-4.4', fill: 'none', stroke: 'var(--ink-2)', 'stroke-width': 1.5, 'stroke-linecap': 'round', 'stroke-linejoin': 'round' }, svg);
    put(svg, r, 9, 13, 28, 'factor', { scale: 0.66 });
    put(svg, r, 9, 47, 28, 'move', 'kept', { scale: 0.66 });
  } else {
    // A square rounds into a circle one frame forward, and leaves a dashed ghost.
    const from = put(svg, r, 9, 12, 32, 'factor', 'rejected', 'faded', { scale: 0.66 });
    const to = { ...NUMBER, ...look('move', 'kept'), x: 46, y: 18, scale: 0.66 };
    joined(svg, r, 'hop', from, to);
    put(svg, r, 9, 46, 18, 'move', 'kept', { scale: 0.66 });
  }
  return svg;
}

function tagSample(r) {
  if (r === 'raised') return null;
  const svg = svgBox();
  put(svg, r, 19, 28, 28, 'move', 'player');
  setTag(tagMark(svg, 1), { ...TAG, x: 13, y: 11 });
  return svg;
}

function lyingSample(r) {
  if (r === 'flat') return null;
  return one(7, 'move', 'lying')(r);
}

function columnsSample(r) {
  const svg = svgBox();
  const left = put(svg, r, 6, 15, 20, 'factor', { scale: 0.6 });
  const right = put(svg, r, 3, 45, 20, 'factor', { scale: 0.6 });
  joined(svg, r, 'thin', left, right);
  for (const [x, k] of [[13, 2], [47, 1]]) {
    const t = el('text', { x, y: 44, 'text-anchor': 'middle', class: 'v-text v-text-name' }, svg);
    t.style.fontSize = '8px';
    t.textContent = `layer ${k}`;
  }
  return svg;
}

/** Every entry: its name, what it means, when to use it, and how to draw it in a rendering (null where a
 *  rendering has no such mark).  core marks the entries the chapter's own legend shows. */
export const ENTRIES = [
  { key: 'move', name: 'Circle', words: 'a source, a number above n/2, or a move', use: 'Every number that is or could be a move.', draw: one(21, 'move'), core: true },
  {
    key: 'factor', name: 'Square', words: 'a number of the bottom half, up to n/2',
    use: 'Every number up to n/2 that the player does not play, from the sources of chapter 3 on.',
    draw: one(7, 'factor'), core: true,
  },
  {
    key: 'plain', name: 'Rounded square',
    words: 'a plain number: one the story has not yet sorted into the sources or the bottom half',
    use: 'Before the sources of chapter 3: the pot and the layers of game 21, and the games of the introduction and ' +
      'chapter 1.',
    draw: one(14, 'plain'), core: true,
  },
  { key: 'kept', name: 'Gray fill', words: 'kept: a source descending greedy keeps, or a move the player plays', use: 'Once a figure has decided a number is in the set.', draw: one(19, 'move', 'kept'), core: true },
  { key: 'rejected', name: 'Hollow and dashed', words: 'rejected: a source the player does not play', use: 'A source descending greedy drops, or a move the player leaves out. Faded, it is a number the game leaves out: a dashed circle above n/2, a square at or below it; with a halo, a move the player gives up.', draw: one(17, 'move', 'rejected'), core: true },
  { key: 'candidate', name: 'Double outline', words: 'a candidate: a factor the player could promote', use: 'A factor whose frame can spare it and whose frame in front can take it on.', draw: one(9, 'factor', 'candidate'), core: true },
  { key: 'promotion', name: 'Promotion', words: 'a factor becomes a move, one frame forward', use: 'The square becomes a circle: flat, it rounds and hops, leaving a ghost, along a dotted arrow, the only arrow the figures draw; raised, it turns over.', draw: promotionSample, core: true },
  { key: 'pay', name: 'Thick line', words: 'a payment: the factor that pays for a move', use: 'From a move to the one factor it takes as its payment.', draw: pair('pay', [18, 'move', 'kept'], [6, 'factor']), core: true },
  { key: 'thin', name: 'Thin line', words: 'a number and one of its maximal factors', use: 'Joins a number to each maximal factor, running rightward. Raised, it hangs as a string; between the columns of a graph it leaves and arrives level, and crosses no number.', draw: pair('thin', [12, 'move'], [6, 'factor']), core: true },
  { key: 'link', name: 'Dotted line', words: 'a candidate and its maximal factors in the frame in front', use: 'From each candidate to each of its maximal factors in the frame in front, once the story names the candidates. It has no arrowhead.', draw: pair('link', [9, 'factor', 'candidate'], [3, 'factor']), core: true },
  { key: 'frame', name: 'Box', words: 'a frame', use: 'The figures draw it round a frame’s sources and factors. Raised, it is a tray.', draw: frameSample(false), core: true },
  { key: 'closed', name: 'Dashed box', words: 'a closed frame: no factor to spare', use: 'From the moment the story counts its freedoms.', draw: frameSample(true), core: true },
  { key: 'player', name: 'Blue', words: 'the player’s: a move the player takes', use: 'Only for the player and the player’s moves; always with a label or a tray.', draw: one(9, 'move', 'player'), core: true },
  { key: 'taxman', name: 'Orange', words: 'the taxman’s: his tax, and the rest', use: 'Only for the taxman and what he takes; always with a label or a tray.', draw: one(3, 'factor', 'taxman'), core: true },
  { key: 'named', name: 'Halo', words: 'the number the words beside the figure are about', use: 'Point at a number in the prose; a figure also uses it for what its caption names.', draw: one(19, 'move', 'kept', 'named'), core: true },
  { key: 'lying', name: 'Lying flat', words: 'a number the player cannot take now', use: 'Raised figures only, the games the reader plays: the player can take a token that stands, not one that lies flat.', draw: lyingSample, core: true },
  { key: 'tag', name: 'Tag', words: 'the place of a move in the game', use: 'Flat figures only, when the player plays a game in place; a raised game fills the player’s tray in order instead.', draw: tagSample },
  { key: 'columns', name: 'Layer columns', words: 'a column for each layer, layer 1 at the right', use: 'Every figure that lays a game out by layers or frames; lines run rightward, toward the front.', draw: columnsSample, core: true },
];

/** Draw the legend into host: a row an entry, the flat mark, the raised one, and its meaning. */
export function mountLegend(host, { all = false } = {}) {
  host.classList.add('legend');
  host.setAttribute('role', 'list');
  const head = document.createElement('div');
  head.className = 'legend-head';
  head.setAttribute('aria-hidden', 'true');
  head.innerHTML = '<span>flat</span><span>raised</span><span>means</span>';
  host.appendChild(head);
  const second = head.cloneNode(true);
  second.classList.add('legend-head-2');
  host.appendChild(second);
  for (const entry of ENTRIES.filter((e) => all || e.core)) {
    const item = document.createElement('div');
    item.className = 'legend-item';
    item.setAttribute('role', 'listitem');
    for (const r of ['flat', 'raised']) {
      const pic = entry.draw(r);
      item.appendChild(pic ?? Object.assign(document.createElement('span'), { className: 'legend-none', textContent: '' }));
    }
    const text = document.createElement('div');
    text.innerHTML = `<b>${entry.name}</b><span>${entry.words}</span>`;
    item.appendChild(text);
    host.appendChild(item);
  }
}
