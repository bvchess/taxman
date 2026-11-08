// How far above opt(n) the two upper bounds sit at the root of every game, from data/bounds.json (the prover's files)
// and data/games.json (opt(n)).  The gap is drawn as a percentage of opt(n), on a scale from 0 to 0.5%.  A game whose
// gap is off the scale gets a small triangle at the top edge, and its tooltip gives the real value.
//
// The parity bound is the point of the chart, so it is drawn in the search's green, heavier; Franklín and Moniot's bound
// is context, in gray.  Neither is the player's blue or the taxman's orange.

import { el, fmt } from '../js/vocab.js';
import { freshSVG, keyboard, responsive, tableView, tooltip } from '../js/chart.js';

const TOP = 0.5; // the top of the scale, in percent
const STEP = 0.1; // the ticks, in percent
const FM_NAME = "Franklín and Moniot's bound";
const PARITY_NAME = 'The parity bound';
const FM_COLOR = 'var(--ink-3)';
const PARITY_COLOR = 'var(--search)';

const pctText = (x, digits = 3) => `${x.toFixed(digits)}%`;

/** For games 2 to lastN, each { n, opt, fm, parity, fmPct, parityPct }, the gaps in percent of opt(n). */
export function boundRows(games, bounds) {
  const b = bounds.columns;
  const rows = b.n.map((n, i) => {
    const opt = games.columns.score[n - 1];
    return { n, opt, fm: b.fm[i], parity: b.parity[i], fmPct: 100 * (b.fm[i] / opt - 1), parityPct: 100 * (b.parity[i] / opt - 1) };
  });
  if (rows[rows.length - 1].n !== games.lastN) throw new Error('the bounds do not reach the last game');
  return rows;
}

/** The numbers the page says of the chart. */
export function boundFacts(games, bounds) {
  const rows = boundRows(games, bounds);
  const last = rows[rows.length - 1];
  const firstInexact = rows.find((r) => r.parity !== r.opt).n;
  const fmOver = rows.filter((r) => r.fmPct > TOP);
  const parityOver = rows.filter((r) => r.parityPct > TOP);
  const over = [...fmOver, ...parityOver].map((r) => r.n);
  return {
    lastN: last.n, exactTo: firstInexact - 1, fmOver: fmOver.length, parityOver: parityOver.length,
    overLast: Math.max(...over), fmGapLast: last.fm - last.opt, parityGapLast: last.parity - last.opt,
    fmPctLast: last.fmPct, parityPctLast: last.parityPct,
  };
}

export function mountBoundsChart(host, { games, bounds }) {
  const rows = boundRows(games, bounds);
  const facts = boundFacts(games, bounds);
  const first = rows[0].n;
  const last = facts.lastN;
  const box = host.querySelector('.chart-box');
  const tip = tooltip(box);
  const series = [
    { key: 'fmPct', name: FM_NAME, color: FM_COLOR, width: 1.2 },
    { key: 'parityPct', name: PARITY_NAME, color: PARITY_COLOR, width: 1.8 },
  ];

  responsive(box, (width) => {
    const narrow = width < 520;
    const m = { l: 44, r: narrow ? 12 : 20, t: 14, b: 32 };
    const plotH = narrow ? 200 : 240;
    const height = m.t + plotH + m.b;
    const plotW = width - m.l - m.r;
    const svg = freshSVG(box, width, height, `Chart of how far above opt(n) two upper bounds sit at the root of each game, as a percentage of opt(n), for every game from ${first} to ${fmt(last)}. The parity bound ends ${pctText(facts.parityPctLast)} above opt(n) and Franklín and Moniot's bound ${pctText(facts.fmPctLast)}. The numbers are in the table below.`);
    const x = (n) => m.l + ((n - first) / (last - first)) * plotW;
    const nAt = (px) => first + ((px - m.l) / plotW) * (last - first);
    const y = (p) => m.t + (1 - Math.min(p, TOP) / TOP) * plotH;
    const axisY = m.t + plotH;

    for (let k = 0; k <= Math.round(TOP / STEP); k++) {
      const p = k * STEP;
      el('line', { x1: m.l, x2: m.l + plotW, y1: y(p), y2: y(p), class: k === 0 ? 'base' : 'grid' }, svg);
      const t = el('text', { x: m.l - 8, y: y(p) + 4, 'text-anchor': 'end', class: 'tick' }, svg);
      t.textContent = pctText(p, 1);
    }
    const step = plotW < 420 ? 2000 : 1000;
    for (let n = 0; n <= last; n += step) {
      const nn = Math.max(first, n);
      el('line', { x1: x(nn), x2: x(nn), y1: axisY, y2: axisY + 5, class: 'base' }, svg);
      const t = el('text', { x: x(nn), y: axisY + 18, 'text-anchor': 'middle', class: 'tick' }, svg);
      t.textContent = fmt(nn);
    }

    // The lines, the gray under the green; a value off the scale is held at the top edge.
    for (const s of series) {
      const d = rows.map((r, i) => `${i ? 'L' : 'M'}${x(r.n).toFixed(1)} ${y(r[s.key]).toFixed(1)}`).join('');
      el('path', { d, fill: 'none', 'stroke-width': s.width, 'stroke-linejoin': 'round', style: `stroke: ${s.color}` }, svg);
    }
    // A small triangle at the top edge for each game off the scale.
    for (const s of series) {
      const d = rows.filter((r) => r[s.key] > TOP)
        .map((r) => `M${x(r.n).toFixed(1)} ${m.t - 5}l3.5 7h-7z`).join('');
      el('path', { d, style: `fill: ${s.color}` }, svg);
    }

    // The labels at the right end, each above its line's gaps there, at a height the line stays below from game 2,500.
    const label = (s, at) => {
      const t = el('text', {
        x: m.l + plotW, y: y(at), 'text-anchor': 'end', class: 'row-label', style: `fill: ${s.color === FM_COLOR ? 'var(--ink-2)' : s.color}`,
      }, svg);
      t.textContent = s.name;
    };
    label(series[0], 0.44);
    label(series[1], 0.1);

    // The hover layer.
    const cross = el('line', { y1: m.t, y2: axisY, class: 'cross', opacity: 0 }, svg);
    const dots = series.map((s) => el('circle', { r: 3.5, 'stroke-width': 2, opacity: 0, style: `fill: ${s.color}; stroke: var(--paper)` }, svg));
    const hit = el('rect', { x: m.l, y: m.t - 8, width: plotW, height: plotH + 8, fill: 'transparent', class: 'hit', 'aria-label': 'Explore the games with the arrow keys' }, svg);
    const showGame = (j) => {
      const r = rows[j];
      const px = x(r.n);
      cross.setAttribute('x1', px);
      cross.setAttribute('x2', px);
      cross.setAttribute('opacity', 1);
      series.forEach((s, k) => {
        dots[k].setAttribute('cx', px);
        dots[k].setAttribute('cy', y(r[s.key]));
        dots[k].setAttribute('opacity', 1);
      });
      tip.show([`Game ${fmt(r.n)}`, [FM_NAME, `${pctText(r.fmPct)} above opt(n)`], [PARITY_NAME, `${pctText(r.parityPct)} above opt(n)`]], px, m.t + 40);
    };
    const hide = () => {
      cross.setAttribute('opacity', 0);
      dots.forEach((d) => d.setAttribute('opacity', 0));
      tip.hide();
    };
    hit.addEventListener('pointermove', (e) => {
      const r = svg.getBoundingClientRect();
      showGame(Math.min(rows.length - 1, Math.max(0, Math.round(nAt(e.clientX - r.left)) - first)));
    });
    hit.addEventListener('pointerleave', hide);
    keyboard(hit, { count: rows.length, onIndex: showGame, onLeave: hide });
  });

  // The table view: each bound's median and highest gap by stretches of 500 games.
  const tableRows = [];
  const median = (v) => v[Math.floor(v.length / 2)];
  for (let a = 0; a < last; a += 500) {
    const lo = Math.max(first, a + 1);
    const hi = Math.min(last, a + 500);
    const part = rows.filter((r) => r.n >= lo && r.n <= hi);
    if (!part.length) continue;
    const cells = ['fmPct', 'parityPct'].flatMap((k) => {
      const v = part.map((r) => r[k]).sort((p, q) => p - q);
      return [pctText(median(v)), pctText(v[v.length - 1])];
    });
    tableRows.push([`games ${fmt(lo)} to ${fmt(hi)}`, ...cells]);
  }
  tableView(host, 'The numbers behind the chart',
    ['stretch', `${FM_NAME}, median`, 'highest', 'The parity bound, median', 'highest'], tableRows);
  return facts;
}
