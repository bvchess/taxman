// The chain of bounds.  Each game's best score bounds the next one's, so the games hang together like the links of a
// chain: game n's link runs from opt(n − 1) up to its ceiling, opt(n − 1) + n, and its dot is opt(n), the start of
// the next link.  A dot at the top of its link is a game at its ceiling.  One game is selected, 12 unless the page's
// link parameter says otherwise, and the labels and the caption describe it.  A flat figure, from data/small.json.

import { el, fmt } from '../js/vocab.js';
import { responsive } from '../js/chart.js';
import { holdHeight } from './hold-height.js';

export const FIRST = 2;
export const LAST = 16;
const START = 12;
const TOP = 100; // the best score at the top of the chart

/** The links of games 2 to `to`, from the recorded games: [{ n, from, ceiling, opt, atCeiling }], from being
 *  opt(n − 1), ceiling opt(n − 1) + n and opt opt(n).  Throws unless from < opt ≤ ceiling for every game. */
export function linksFacts(small, to = LAST) {
  const facts = [];
  for (let n = FIRST; n <= to; n++) {
    const prev = small[n - 2];
    const cur = small[n - 1];
    if (prev?.n !== n - 1 || cur?.n !== n) throw new Error(`links of ${n}: small.json has no games n − 1 and n`);
    const ceiling = prev.score + n;
    const link = { n, from: prev.score, ceiling, opt: cur.score, atCeiling: cur.score === ceiling };
    if (!(link.from < link.opt && link.opt <= link.ceiling)) {
      throw new Error(`links of ${n}: opt(${n}) = ${link.opt} is not above ${link.from} and at most ${link.ceiling}`);
    }
    facts.push(link);
  }
  return facts;
}

/** The caption's words for one game. */
export function linkWords({ n, from, ceiling, opt, atCeiling }) {
  return `Game ${n}'s link runs from opt(${n - 1}) = ${fmt(from)} up to its ceiling, ${fmt(from)} + ${n} = `
    + `${fmt(ceiling)}, and opt(${n}) = ${fmt(opt)}${atCeiling ? ' is at the top' : ''}.`;
}

const MARGIN = { left: 46, right: 14, top: 12, bottom: 40 };
const TICKS = [0, 20, 40, 60, 80, 100];
const LABEL_SIZE = 12;
const GAP = 7; // between a mark and its label
const MAG_W = 16; // the magnified bar's width
const MAG_DOT = 4.8;
const BOX_PAD = 6; // inside the inset's box

/** Mount the figure in host, which holds an <svg> and a .fig-caption.  Returns the links' facts and select(n). */
export function mountLinks(host, { small }) {
  const facts = linksFacts(small);
  const svg = host.querySelector('svg');
  const caption = host.querySelector('.fig-caption');
  const words = facts.map(linkWords);
  caption.setAttribute('aria-live', 'polite');
  holdHeight(caption, words);
  svg.setAttribute('tabindex', '0');

  const asked = Number(new URLSearchParams(location.search).get('link'));
  let selected = facts.some((f) => f.n === asked) ? asked : START;
  let marks = null; // what the current drawing holds: { bars, labels, ... }

  function draw(width) {
    const narrow = width < 520;
    const height = Math.round(Math.min(360, Math.max(270, width * 0.52)));
    svg.replaceChildren();
    svg.setAttribute('viewBox', `0 0 ${width} ${height}`);
    svg.setAttribute('width', width);
    svg.setAttribute('height', height);
    const left = MARGIN.left;
    const right = width - MARGIN.right;
    const base = height - MARGIN.bottom;
    const top = MARGIN.top;
    const pad = narrow ? 8 : 14;
    const step = (right - left - 2 * pad) / (LAST - 1);
    const X = (game) => left + pad + (game - 1) * step;
    const Y = (score) => base - (score / TOP) * (base - top);
    const barW = Math.min(narrow ? 8 : 12, step * 0.5);
    const dotR = narrow ? 3.2 : 4.2;

    const axes = el('g', { class: 'axis' }, svg);
    for (const t of TICKS) {
      el('line', { x1: left, x2: right, y1: Y(t), y2: Y(t), class: t === 0 ? 'base' : 'grid' }, axes);
      el('text', { x: left - 8, y: Y(t) + 4, 'text-anchor': 'end', class: 'tick' }, axes).textContent = t;
    }
    for (let game = 1; game <= LAST; game++) {
      if (step < 18 && game > 1 && game % 2) continue;
      el('text', { x: X(game), y: base + 17, 'text-anchor': 'middle', class: 'tick' }, axes).textContent = game;
    }
    el('text', { x: (left + right) / 2, y: height - 6, 'text-anchor': 'middle' }, axes).textContent = 'game';
    el('text', {
      transform: `translate(12 ${(top + base) / 2}) rotate(-90)`, 'text-anchor': 'middle',
    }, axes).textContent = 'best score';

    // The wires first, so that links and dots lie over them: each dot hangs the next link from its own height.
    const wires = el('g', { class: 'link-wires' }, svg);
    for (const f of facts) {
      el('line', { x1: X(f.n - 1), x2: X(f.n), y1: Y(f.from), y2: Y(f.from), class: 'link-wire' }, wires);
    }
    const bars = new Map();
    for (const f of facts) {
      bars.set(f.n, el('rect', {
        x: X(f.n) - barW / 2, y: Y(f.ceiling), width: barW, height: Y(f.from) - Y(f.ceiling), rx: barW / 2,
        class: 'link-bar',
      }, svg));
    }
    // The chain starts at game 1 with 0, and each later game's dot ends its link.
    const dots = [{ n: 1, opt: 0 }, ...facts].map((f) => el('circle', {
      cx: X(f.n), cy: Y(f.opt), r: dotR, class: 'link-dot',
    }, svg));
    const labels = el('g', { class: 'link-labels', 'aria-hidden': 'true' }, svg);
    for (const f of facts) {
      const hit = el('rect', { x: X(f.n) - step / 2, y: top, width: step, height: base - top, class: 'link-hit' }, svg);
      hit.addEventListener('pointerenter', () => select(f.n));
      hit.addEventListener('pointerdown', () => select(f.n));
    }
    marks = { bars, labels, left, top, base, pad, dots: dots.map((d, i) => [i + 1, d]) };
    paint();
  }

  // The selected link, magnified in the empty upper left of the plot, where the chain has not yet climbed: its bar
  // with the dot at the same share of its height, and the labels level with the marks, to the right.  A dot close to
  // the top or the foot has its label nudged, so that the lines stay a line apart.
  function paint() {
    const { bars, labels, left, top, base, pad } = marks;
    const f = facts.find((g) => g.n === selected);
    for (const [n, bar] of bars) bar.classList.toggle('is-selected', n === selected);
    for (const [n, dot] of marks.dots) dot.classList.toggle('is-selected', n === selected);
    labels.replaceChildren();
    const lineH = LABEL_SIZE * 1.25;
    const h = Math.min(0.42 * (base - top), 120);
    const yTop = top + BOX_PAD + 26;
    const yFoot = yTop + h;
    const yDot = yFoot - ((f.opt - f.from) / f.n) * h;
    const x = left + pad + BOX_PAD + MAG_W / 2;
    const word = (text, y, cls = 'link-label') => {
      const t = el('text', { x: x + MAG_W / 2 + GAP, y: y + LABEL_SIZE / 3, class: cls }, labels);
      t.textContent = text;
    };
    const title = el('text', { x: x - MAG_W / 2, y: top + BOX_PAD + 10, class: 'link-label link-title' }, labels);
    title.textContent = `Game ${f.n}`;
    el('rect', {
      x: x - MAG_W / 2, y: yTop, width: MAG_W, height: h, rx: MAG_W / 2, class: 'link-bar is-selected',
    }, labels);
    el('circle', { cx: x, cy: yDot, r: MAG_DOT, class: 'link-dot is-selected' }, labels);
    word(`ceiling ${fmt(f.ceiling)}`, yTop);
    word(`opt(${f.n}) = ${fmt(f.opt)}`, Math.min(Math.max(yDot, yTop + lineH), yFoot - lineH));
    word(`opt(${f.n - 1}) = ${fmt(f.from)}`, yFoot);
    // The box behind it, with its paper fill, says that the inset is not data.
    const at = labels.getBBox();
    labels.prepend(el('rect', {
      x: at.x - BOX_PAD, y: at.y - BOX_PAD, width: at.width + 2 * BOX_PAD, height: at.height + 2 * BOX_PAD, rx: 6,
      class: 'link-box',
    }));
    caption.textContent = words[f.n - FIRST];
  }

  function select(n) {
    if (n === selected || n < FIRST || n > LAST) return;
    selected = n;
    paint();
  }

  svg.addEventListener('keydown', (e) => {
    const to = { ArrowRight: selected + 1, ArrowUp: selected + 1, ArrowLeft: selected - 1, ArrowDown: selected - 1,
      Home: FIRST, End: LAST }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    select(to);
  });

  responsive(host, draw);
  return { facts, select };
}
