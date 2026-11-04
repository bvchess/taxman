// The walk up the chain, game by game: the board of game n, drawn as figures/arrival.js draws it, and under it a strip
// of the games from 2 to 100, each cell shaded by the rule that settles it (js/proof.js), which the reader drags
// along or steps with the two buttons.  Each step moves the board from game n − 1 to game n: n appears after the
// numbers before it, the moves that change wear a halo, and the caption says what happened and how the chain proved it.
// The caption keeps the height of its longest message, so nothing below the figure moves while the reader walks.
//
// URL parameter: walk=N opens the figure at game N.

import { scene } from '../js/scene.js';
import { el } from '../js/vocab.js';
import { responsive } from '../js/chart.js';
import { COLS, arrival, arrivalWords, boardLooks, boardSize, cell } from './arrival.js';
import { RULE } from '../js/proof.js';
import { legendHTML, ruleCell, swatch } from './settled.js';

export function mountWalker(host, data, { from = 2, to = 100, start = 22, param = 'walk' } = {}) {
  const steps = [];
  for (let n = from; n <= to; n++) steps.push(arrival(data, n));
  const rows = Math.ceil(to / COLS);
  const { width, height } = boardSize(rows);
  const sc = scene(host.querySelector('svg'), {
    width, height,
    label: 'The board of the game the strip below names, ten numbers to a row from 1 at the top left. The moves of '
      + 'its optimal game are blue circles, and what changed from the game before wears a halo.',
  });
  // The places of the numbers still to come, as faint dots under the marks, so that the reader sees the board fill.
  for (let x = 1; x <= to; x++) {
    const at = cell(x);
    el('circle', { cx: at.x, cy: at.y, r: 1.8, class: 'walk-slot' }, sc.layer.box);
  }
  for (let x = 1; x <= to; x++) sc.number(x, x);
  sc.states(steps.map((f) => boardLooks(f)));

  const caption = host.querySelector('.walk-caption');
  const words = (f) => `<span class="walk-n">${swatch(f.rule)}Game ${f.n}</span> ${arrivalWords(f, { record: true })}`;

  // The legend of the strip: the rules its games use.
  const used = new Set(steps.map((f) => f.rule));
  host.querySelector('.rule-legend').innerHTML = legendHTML((key) => used.has(key));

  // The strip: a slider whose track is the games themselves.
  const strip = host.querySelector('.walk-strip');
  strip.setAttribute('role', 'slider');
  strip.setAttribute('tabindex', '0');
  strip.setAttribute('aria-label', 'Game');
  strip.setAttribute('aria-valuemin', String(from));
  strip.setAttribute('aria-valuemax', String(to));
  let n = start;
  let layout = null;
  let thumb = null;
  responsive(strip, (w) => {
    strip.querySelector('svg')?.remove();
    const pitch = w / steps.length;
    const h = 38;
    const svg = el('svg', { viewBox: `0 0 ${w} ${h}`, width: w, height: h, 'aria-hidden': 'true' });
    strip.prepend(svg);
    const size = Math.max(1, pitch - (pitch >= 6 ? 2 : 1));
    const cellY = 5;
    const cellH = 18;
    steps.forEach((f, k) => {
      ruleCell(svg, f.rule, k * pitch + (pitch - size) / 2, cellY, size, cellH);
    });
    const every = pitch * 10 >= 34 ? 10 : 20;
    for (let t = Math.ceil(from / every) * every; t <= to; t += every) {
      const x = (t - from + 0.5) * pitch;
      el('line', { x1: x, x2: x, y1: cellY + cellH + 2, y2: cellY + cellH + 5, class: 'walk-tick' }, svg);
      const label = el('text', { x, y: h - 1, 'text-anchor': 'middle', class: 'walk-tick-label' }, svg);
      label.textContent = String(t);
    }
    thumb = el('rect', { y: 1, height: cellH + 8, rx: 3, class: 'walk-thumb' }, svg);
    layout = { pitch, size };
    place();
  });
  function place() {
    if (!layout || !thumb) return;
    const k = n - from;
    const wThumb = Math.max(layout.pitch + 4, 8);
    thumb.setAttribute('x', (k + 0.5) * layout.pitch - wThumb / 2);
    thumb.setAttribute('width', wThumb);
  }

  // The caption is as tall as its longest message at the present width.
  const measure = document.createElement('div');
  measure.className = 'walk-caption walk-measure';
  measure.setAttribute('aria-hidden', 'true');
  host.appendChild(measure);
  let measuredAt = 0;
  const fitCaption = () => {
    const w = caption.clientWidth;
    if (!w || w === measuredAt) return;
    measuredAt = w;
    measure.style.width = `${w}px`;
    let most = 0;
    for (const f of steps) {
      measure.innerHTML = words(f);
      most = Math.max(most, measure.getBoundingClientRect().height);
    }
    caption.style.minHeight = `${Math.ceil(most)}px`;
  };
  new ResizeObserver(fitCaption).observe(caption);

  function go(next, { animate = true } = {}) {
    n = Math.min(to, Math.max(from, Math.round(next)));
    const f = steps[n - from];
    caption.innerHTML = words(f);
    strip.setAttribute('aria-valuenow', String(n));
    strip.setAttribute('aria-valuetext', `Game ${n}, ${RULE[f.rule].name}`);
    host.querySelector('[data-act="prev"]').disabled = n === from;
    host.querySelector('[data-act="next"]').disabled = n === to;
    place();
    if (animate) {
      sc.go(n - from);
      const url = new URL(location.href);
      url.searchParams.set(param, String(n));
      history.replaceState(null, '', url);
    } else sc.show(n - from);
  }

  host.querySelector('[data-act="prev"]').addEventListener('click', () => go(n - 1));
  host.querySelector('[data-act="next"]').addEventListener('click', () => go(n + 1));
  strip.addEventListener('keydown', (e) => {
    const next = {
      ArrowLeft: n - 1, ArrowDown: n - 1, ArrowRight: n + 1, ArrowUp: n + 1, PageDown: n - 10, PageUp: n + 10, Home: from,
      End: to,
    }[e.key];
    if (next === undefined) return;
    e.preventDefault();
    go(next);
  });
  const nAt = (e) => {
    const r = strip.getBoundingClientRect();
    return from + Math.floor(((e.clientX - r.left) / r.width) * steps.length);
  };
  strip.addEventListener('pointerdown', (e) => {
    strip.setPointerCapture(e.pointerId);
    go(nAt(e));
  });
  strip.addEventListener('pointermove', (e) => {
    if (strip.hasPointerCapture(e.pointerId) && nAt(e) !== n) go(nAt(e));
  });

  const asked = Number(new URLSearchParams(location.search).get(param));
  go(Number.isInteger(asked) && asked >= from && asked <= to ? asked : start, { animate: false });
  fitCaption();

  return {
    steps,
    goState: (state) => go(Number(state)),
    stopName: (state) => `game ${state}`,
    highlight: (x, on) => sc.highlight(x, on),
  };
}
