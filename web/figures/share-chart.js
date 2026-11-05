// The player's share of the board in every game of the chain, opt(n) / (1 + 2 + ... + n), against Hensley's bounds on
// its limit: more than 1/2, and at most 35/48 (D. Hensley, "A winning strategy at Taxman", The Fibonacci Quarterly 26
// (1988) 262-270, the theorem on p. 263 and the bound C <= 35/48 on p. 266).  All from data/games.json.
//
// The games run along a scale of n that grows tenfold from one labeled mark to the next, so that both the first games,
// where the share swings widely, and the last thousands, where it has settled, can be seen.  A pixel's column that holds
// one game draws it as a dot; one that holds several draws the range of their shares.

import { el, fmt } from '../js/vocab.js';
import { freshSVG, keyboard, pct, responsive, tableView, tooltip } from '../js/chart.js';

/** Hensley's bounds on lim opt(n) / (n²/2), which is also the limit of the share. */
export const HENSLEY = { lower: 1 / 2, upper: 35 / 48 };

/** The share of game i (0-based) of the columns c: its score over 1 + 2 + ... + n. */
export const shareOf = (c, i) => c.score[i] / ((c.n[i] * (c.n[i] + 1)) / 2);

/** The games n whose share is at most 1/2: those in which the player does not beat the taxman. */
export const notWon = (c) => c.n.filter((n, i) => 2 * c.score[i] <= (n * (n + 1)) / 2);

/** The lowest and highest share over the games from a to b. */
export function shareRange(c, a, b) {
  let lo = Infinity;
  let hi = -Infinity;
  for (let i = a - 1; i < b; i++) {
    lo = Math.min(lo, shareOf(c, i));
    hi = Math.max(hi, shareOf(c, i));
  }
  return [lo, hi];
}

export function mountShareChart(host, { games }) {
  const c = games.columns;
  const last = games.lastN;
  const first = 2; // game 1 has no move and a share of 0
  const endShare = shareOf(c, last - 1);
  const box = host.querySelector('.chart-box');
  const tip = tooltip(box);
  host.querySelector('.chart-sub').textContent = `Every game from game ${fmt(first)} to game ${fmt(last)}, along a scale of n that grows tenfold from one labeled mark to the next. Where games crowd together, a mark spans their shares.`;

  responsive(box, (width) => {
    const narrow = width < 560;
    const m = { l: narrow ? 38 : 46, r: narrow ? 12 : 22, t: 16, b: 52 };
    const plotH = narrow ? 230 : 290;
    const height = m.t + plotH + m.b;
    const plotW = width - m.l - m.r;
    const svg = freshSVG(box, width, height, `Chart of the player's share of the board in every game from ${first} to ${last}, on a logarithmic scale of n, between Hensley's bounds of 1/2 and 35/48. The share settles near ${pct(endShare)}. The numbers are in the table below.`);
    const lx0 = Math.log(first);
    const lx1 = Math.log(last);
    const x = (n) => m.l + ((Math.log(n) - lx0) / (lx1 - lx0)) * plotW;
    const nAt = (px) => Math.min(last, Math.max(first, Math.round(Math.exp(lx0 + ((px - m.l) / plotW) * (lx1 - lx0)))));
    const lo = 0.46;
    const hi = 0.75;
    const y = (s) => m.t + (1 - (s - lo) / (hi - lo)) * plotH;

    // Hensley's range for the limit, a wash between his two bounds, each a dashed line with its name.
    el('rect', { x: m.l, y: y(HENSLEY.upper), width: plotW, height: y(HENSLEY.lower) - y(HENSLEY.upper), class: 'share-range' }, svg);
    for (let s = 0.5; s <= hi + 1e-9; s += 0.05) {
      el('line', { x1: m.l, x2: m.l + plotW, y1: y(s), y2: y(s), class: 'grid' }, svg);
      const t = el('text', { x: m.l - 8, y: y(s) + 4, 'text-anchor': 'end', class: 'tick' }, svg);
      t.textContent = pct(s, 0);
    }
    // Each bound's name sits above its line at the right, where the games have settled far from both.
    const bound = (s, words, sub) => {
      el('line', { x1: m.l, x2: m.l + plotW, y1: y(s), y2: y(s), class: 'share-bound' }, svg);
      const t = el('text', { x: m.l + plotW, y: y(s) - (sub ? 23 : 7), 'text-anchor': 'end', class: 'row-label' }, svg);
      t.textContent = words;
      if (sub) {
        const u = el('text', { x: m.l + plotW, y: y(s) - 7, 'text-anchor': 'end', class: 'row-count' }, svg);
        u.textContent = sub;
      }
    };
    bound(HENSLEY.upper, narrow ? 'upper bound, 35/48' : 'Hensley’s upper bound, 35/48');
    bound(HENSLEY.lower, narrow ? 'lower bound, 1/2' : 'Hensley’s lower bound, 1/2', 'above it, the player beats the taxman');

    // The games: a dot for a column with one game, the range of the shares for a column with several.
    let path = '';
    let i = first - 1;
    for (let px = 0; px < plotW; px++) {
      let a = Infinity;
      let b = -Infinity;
      let k = 0;
      while (i < last && x(c.n[i]) < m.l + px + 1) {
        a = Math.min(a, shareOf(c, i));
        b = Math.max(b, shareOf(c, i));
        i++;
        k++;
      }
      if (!k) continue;
      const cx = (m.l + px + 0.5).toFixed(1);
      if (k === 1) el('circle', { cx, cy: y(a).toFixed(1), r: 2.6, class: 'share-dot' }, svg);
      else path += `M${cx} ${y(b).toFixed(1)}V${Math.max(y(b) + 1.5, y(a)).toFixed(1)}`;
    }
    el('path', { d: path, class: 'share-band' }, svg);
    el('circle', { cx: x(last), cy: y(endShare), r: 4.5, 'stroke-width': 2, style: 'fill: var(--player); stroke: var(--paper)' }, svg);
    const endLabel = el('text', { x: x(last) - 8, y: y(endShare) - 12, 'text-anchor': 'end', class: 'row-label' }, svg);
    endLabel.textContent = `${pct(endShare)} at game ${fmt(last)}`;

    // The n axis: a labeled mark at each power of ten and at the ends, and small marks between.
    const axisY = m.t + plotH;
    el('line', { x1: m.l, x2: m.l + plotW, y1: axisY + 0.5, y2: axisY + 0.5, class: 'base' }, svg);
    for (let decade = 1; decade <= last; decade *= 10) {
      for (let k = 1; k < 10; k++) {
        const n = k * decade;
        if (n < first || n > last) continue;
        const major = k === 1;
        el('line', { x1: x(n), x2: x(n), y1: axisY, y2: axisY + (major ? 6 : 3), class: 'base' }, svg);
      }
    }
    const labels = [first, 10, 100, 1000, last].filter((n, j, all) => n <= last && all.indexOf(n) === j);
    for (const n of labels) {
      if (n !== last && x(last) - x(n) < 34) continue;
      const t = el('text', { x: x(n), y: axisY + 19, 'text-anchor': 'middle', class: 'tick' }, svg);
      t.textContent = fmt(n);
    }
    const name = el('text', { x: m.l + plotW / 2, y: axisY + 40, 'text-anchor': 'middle', class: 'row-count' }, svg);
    name.textContent = 'game n';

    // The hover layer: a crosshair that finds the game under the pointer.
    const cross = el('line', { y1: m.t, y2: axisY, class: 'cross', opacity: 0 }, svg);
    const dot = el('circle', { r: 4, 'stroke-width': 2, opacity: 0, style: 'fill: var(--player); stroke: var(--paper)' }, svg);
    const hit = el('rect', { x: m.l, y: m.t, width: plotW, height: plotH, fill: 'transparent', class: 'hit', 'aria-label': 'Explore the shares with the arrow keys' }, svg);
    const showGame = (n) => {
      const j = n - 1;
      const px = x(n);
      cross.setAttribute('x1', px);
      cross.setAttribute('x2', px);
      cross.setAttribute('opacity', 1);
      dot.setAttribute('cx', px);
      dot.setAttribute('cy', y(Math.max(lo, Math.min(hi, shareOf(c, j)))));
      dot.setAttribute('opacity', 1);
      tip.show([`Game ${fmt(n)}`, ['opt(n)', fmt(c.score[j])], ['1 + 2 + ⋯ + n', fmt((n * (n + 1)) / 2)], ['player’s share', pct(shareOf(c, j), 2)]], px, m.t + 40);
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
    keyboard(hit, { count: last - first + 1, onIndex: (j) => showGame(j + first), onLeave: hide });
  });

  // The table view: the lowest and highest share of each stretch, by powers of ten and then by thousands.
  const stretches = [[first, 9], [10, 99], [100, 999]];
  for (let a = 1000; a <= last; a += 1000) stretches.push([a, Math.min(last, a + 999)]);
  const rows = stretches.filter(([a]) => a <= last).map(([a, b]) => {
    const [lo, hi] = shareRange(c, a, Math.min(b, last));
    return [`games ${fmt(a)} to ${fmt(Math.min(b, last))}`, pct(lo, 2), pct(hi, 2)];
  });
  tableView(host, 'The numbers behind the chart', ['stretch', 'lowest share', 'highest share'], rows);
  return { endShare };
}
