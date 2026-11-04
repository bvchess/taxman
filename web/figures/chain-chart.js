// The whole chain, n = 1 to the last game: the player's share of the board, opt(n) / (1 + 2 + ... + n), and below it
// how each game was proven, one row a proof kind, all from data/games.json.
//
// The share is drawn as the range of its values within each pixel's games, so the zigzag between neighboring games
// shows as a band that narrows.  Each proof row is a small histogram: the share of each stretch's games proven so.

import { el, fmt } from '../js/vocab.js';
import { freshSVG, keyboard, pct, responsive, tableView, tooltip } from '../js/chart.js';

/** The proof kinds of a results line, in plain words (docs/GLOSSARY.md, "proof"). */
export const PROOFS = [
  { key: 'exhausted', name: 'searched the whole tree', short: 'a search of the whole tree' },
  { key: 'same_search', name: 'repeats an earlier search', short: 'a repeat of an earlier game’s search' },
  { key: 'cap', name: 'reaches the ceiling', short: 'the ceiling' },
  { key: 'prime_formula', name: 'prime formula', short: 'the prime formula' },
  { key: 'known', name: 'proven in the bounds store', short: 'a game the bounds store proved' },
  { key: 'trivial', name: 'trivial: nothing to promote', short: 'trivial' },
];


export function mountChainChart(host, { games }) {
  const c = games.columns;
  const last = games.lastN;
  const count = c.n.length;
  const share = c.n.map((n, i) => c.score[i] / ((n * (n + 1)) / 2));
  const kindOf = new Map(PROOFS.map((p) => [p.key, p]));
  const totals = Object.fromEntries(PROOFS.map((p) => [p.key, c.proof.filter((x) => x === p.key).length]));
  const box = host.querySelector('.chart-box');
  const tip = tooltip(box);
  const endShare = share[count - 1];
  host.querySelector('.chart-sub').textContent = `For every game from n = 1 to ${fmt(last)}. The player’s share of the board settles near ${pct(endShare)}; game ${fmt(last)} scores ${fmt(c.score[count - 1])} of ${fmt((last * (last + 1)) / 2)}.`;

  responsive(box, (width) => {
    const narrow = width < 620;
    // On a wide screen the names sit in a column to the left, each kind's count beside it; on a narrow one, above.
    const labelW = narrow ? 0 : 238;
    const m = { l: narrow ? 34 : labelW + 14, r: narrow ? 10 : 18, t: 14 };
    const nameX = labelW - 52;
    const countX = m.l - 8;
    const shareH = narrow ? 140 : 170;
    const rowH = narrow ? 38 : 27;
    const rowGap = narrow ? 16 : 0;
    const top2 = m.t + shareH + 44;
    const height = top2 + PROOFS.length * rowH + 34;
    const plotW = width - m.l - m.r;
    const svg = freshSVG(box, width, height, `Chart of the chain from n = 1 to ${last}: the player's share of the board, and how each game was proven. The numbers are in the table below.`);
    const x = (n) => m.l + ((n - 1) / (last - 1)) * plotW;
    const nAt = (px) => Math.min(last, Math.max(1, Math.round(1 + ((px - m.l) / plotW) * (last - 1))));
    const lo = 0.5;
    const hi = 0.75;
    const y = (s) => m.t + (1 - (s - lo) / (hi - lo)) * shareH;

    // The share: gridlines, then one band of the values each pixel covers.
    const g = el('g', {}, svg);
    for (let s = lo; s <= hi + 1e-9; s += 0.05) {
      el('line', { x1: m.l, x2: m.l + plotW, y1: y(s), y2: y(s), class: 'grid' }, g);
      const t = el('text', { x: m.l - 8, y: y(s) + 4, 'text-anchor': 'end', class: 'tick' }, g);
      t.textContent = pct(s, 0);
    }
    if (!narrow) {
      const label = el('text', { x: nameX, y: m.t + shareH / 2 - 4, class: 'row-label', 'text-anchor': 'end' }, g);
      label.textContent = 'player’s share';
      const sub = el('text', { x: nameX, y: m.t + shareH / 2 + 13, class: 'row-count', 'text-anchor': 'end' }, g);
      sub.textContent = 'opt(n) / (1 + 2 + ... + n)';
    }
    let d = '';
    let i = 1; // game 1 has no move and a share of 0; it sits below the axis and is left out
    for (let px = 0; px < plotW; px++) {
      let a = Infinity;
      let b = -Infinity;
      while (i < count && x(c.n[i]) < m.l + px + 1) {
        a = Math.min(a, share[i]);
        b = Math.max(b, share[i]);
        i++;
      }
      if (a === Infinity) continue;
      const top = y(Math.min(hi, b));
      const bottom = Math.max(top + 1.6, y(Math.max(lo, a)));
      d += `M${m.l + px + 0.5} ${top.toFixed(1)}V${bottom.toFixed(1)}`;
    }
    el('path', { d, 'stroke-width': 1.3, fill: 'none', style: 'stroke: var(--player)' }, g);
    el('circle', { cx: x(last), cy: y(endShare), r: 4.5, 'stroke-width': 2, style: 'fill: var(--player); stroke: var(--paper)' }, g);
    const endLabel = el('text', { x: x(last) - 8, y: y(endShare) - 12, 'text-anchor': 'end', class: 'row-label' }, g);
    endLabel.textContent = `${pct(endShare)} at n = ${fmt(last)}`;

    // How each game was proven: a row a kind, a bar a stretch of games, its height the stretch's share.
    const binPx = 6;
    const bins = Math.max(10, Math.floor(plotW / binPx));
    const per = count / bins;
    PROOFS.forEach((p, r) => {
      const ry = top2 + r * rowH;
      const base = ry + rowH - 4;
      const barH = rowH - (narrow ? 20 : 8);
      el('line', { x1: m.l, x2: m.l + plotW, y1: base + 0.5, y2: base + 0.5, class: 'base' }, svg);
      const name = el('text', { x: narrow ? m.l : nameX, y: narrow ? ry + 11 : base - 4, 'text-anchor': narrow ? 'start' : 'end', class: 'row-label' }, svg);
      name.textContent = p.name;
      const k = el('text', { x: narrow ? m.l + plotW : countX, y: narrow ? ry + 11 : base - 4, 'text-anchor': 'end', class: 'row-count' }, svg);
      k.textContent = fmt(totals[p.key]);
      let path = '';
      for (let b = 0; b < bins; b++) {
        const i0 = Math.floor(b * per);
        const i1 = Math.floor((b + 1) * per);
        let hits = 0;
        for (let j = i0; j < i1; j++) if (c.proof[j] === p.key) hits++;
        if (!hits) continue;
        const h = Math.max(1.5, (hits / (i1 - i0)) * barH);
        const bx = m.l + (b * plotW) / bins;
        path += `M${bx.toFixed(1)} ${base}h${(plotW / bins - 2).toFixed(1)}v${(-h).toFixed(1)}h${(-(plotW / bins - 2)).toFixed(1)}z`;
      }
      el('path', { d: path, style: 'fill: var(--ink-2)' }, svg);
    });
    if (!narrow) {
      const counts = el('text', { x: countX, y: top2 - 8, class: 'row-count', 'text-anchor': 'end' }, svg);
      counts.textContent = 'games';
      const head = el('text', { x: nameX, y: top2 - 8, class: 'row-count', 'text-anchor': 'end' }, svg);
      head.textContent = 'how each game was proven';
    }

    // The n axis, shared by both.
    const axisY = top2 + PROOFS.length * rowH + 4;
    const step = plotW < 420 ? 2000 : 1000;
    for (let n = 0; n <= last; n += step) {
      const nn = Math.max(1, n);
      el('line', { x1: x(nn), x2: x(nn), y1: axisY, y2: axisY + 5, class: 'base' }, svg);
      const t = el('text', { x: x(nn), y: axisY + 18, 'text-anchor': 'middle', class: 'tick' }, svg);
      t.textContent = n === 0 ? '1' : fmt(n);
    }
    el('line', { x1: m.l, x2: m.l + plotW, y1: y(lo) + 0.5, y2: y(lo) + 0.5, class: 'base' }, svg);

    // The hover layer: a crosshair that finds the game under the pointer.
    const cross = el('line', { y1: m.t, y2: axisY, class: 'cross', opacity: 0 }, svg);
    const dot = el('circle', { r: 4, 'stroke-width': 2, opacity: 0, style: 'fill: var(--player); stroke: var(--paper)' }, svg);
    const hit = el('rect', { x: m.l, y: m.t, width: plotW, height: axisY - m.t, fill: 'transparent', class: 'hit', 'aria-label': 'Explore the chain with the arrow keys' }, svg);
    const showGame = (n) => {
      const j = n - 1;
      const px = x(n);
      cross.setAttribute('x1', px);
      cross.setAttribute('x2', px);
      cross.setAttribute('opacity', 1);
      dot.setAttribute('cx', px);
      dot.setAttribute('cy', y(Math.max(lo, Math.min(hi, share[j]))));
      dot.setAttribute('opacity', n > 1 ? 1 : 0);
      tip.show([`Game ${fmt(n)}`, ['opt(n)', fmt(c.score[j])], ['player’s share', pct(share[j], 2)], ['proven by', kindOf.get(c.proof[j])?.short ?? c.proof[j]]], px, m.t + 40);
    };
    const hide = () => {
      cross.setAttribute('opacity', 0);
      dot.setAttribute('opacity', 0);
      tip.hide();
    };
    hit.addEventListener('pointermove', (e) => {
      const r = svg.getBoundingClientRect();
      showGame(nAt(e.clientX - r.left));
    });
    hit.addEventListener('pointerleave', hide);
    keyboard(hit, { count, onIndex: (j) => showGame(j + 1), onLeave: hide });
  });

  // The table view: the numbers behind the chart, by stretches of 500 games.
  const rows = [];
  for (let a = 1; a <= last; a += 500) {
    const b = Math.min(last, a + 499);
    const sl = (col) => col.slice(a - 1, b);
    const kinds = PROOFS.map((p) => fmt(sl(c.proof).filter((x) => x === p.key).length));
    const sh = share.slice(Math.max(1, a - 1), b);
    rows.push([`${fmt(a)} to ${fmt(b)}`, ...kinds, `${pct(Math.min(...sh))} to ${pct(Math.max(...sh))}`]);
  }
  rows.push(['all', ...PROOFS.map((p) => fmt(totals[p.key])), '']);
  tableView(host, 'The numbers behind the chart', ['games', ...PROOFS.map((p) => p.name), 'player’s share'], rows);
}
