// The top half's share of the score: for every game of the chain that was settled by a search of its own, the share
// of its score that its numbers above n/2 carry, from data/games.json.  The recorded game plays exactly U* above n/2,
// so the share is in_the_bag / score.
//
// The share is drawn as the range of its values within each pixel's games, on a scale from 90% to the whole score,
// so that the band's narrow spread shows; the rest of each score, up to 100%, is the promotions.

import { el, fmt } from '../js/vocab.js';
import { freshSVG, keyboard, pct, responsive, tableView, tooltip } from '../js/chart.js';

const LO = 0.9; // the foot of the scale

/** The games settled by a search of their own, each { n, score, top, share }, in order of n. */
export function searchedShares(games) {
  const c = games.columns;
  const out = [];
  for (let i = 0; i < c.n.length; i++) {
    if (c.method[i] !== 'search') continue;
    out.push({ n: c.n[i], score: c.score[i], top: c.inTheBag[i], share: c.inTheBag[i] / c.score[i] });
  }
  return out;
}

/** The numbers the page says of the share: the count of games, the first n, the median share, the last game, the
 *  games of the lowest and the highest share, and the lowest and highest share past game past. */
export function shareFacts(games, past = 500) {
  const pts = searchedShares(games);
  const sorted = [...pts].sort((a, b) => a.share - b.share);
  const later = pts.filter((p) => p.n > past).map((p) => p.share);
  return {
    count: pts.length, first: pts[0].n, last: pts[pts.length - 1], median: sorted[Math.floor(sorted.length / 2)].share,
    low: sorted[0], high: sorted[sorted.length - 1], past, lowLater: Math.min(...later), highLater: Math.max(...later),
  };
}

export function mountShareChart(host, { games }) {
  const pts = searchedShares(games);
  const facts = shareFacts(games);
  const last = games.lastN;
  const box = host.querySelector('.chart-box');
  const tip = tooltip(box);
  host.querySelector('.chart-sub').textContent = `Each of the ${fmt(facts.count)} games of the chain settled by a search, from game ${fmt(facts.first)} to game ${fmt(facts.last.n)}: the share of its score that its numbers above n/2 carry. Past game ${fmt(facts.past)} it stays between ${pct(facts.lowLater)} and ${pct(facts.highLater)}.`;

  responsive(box, (width) => {
    const narrow = width < 520;
    const m = { l: 42, r: narrow ? 12 : 20, t: 16, b: 30 };
    const plotH = narrow ? 160 : 190;
    const height = m.t + plotH + m.b;
    const plotW = width - m.l - m.r;
    if (facts.low.share < LO) throw new Error(`the share chart: game ${facts.low.n} falls below its scale`);
    const svg = freshSVG(box, width, height, `Chart of the share of each searched game's score that its numbers above n/2 carry, against n. It stays near ${pct(facts.median, 0)}. The numbers are in the table below.`);
    const x = (n) => m.l + ((n - 1) / (last - 1)) * plotW;
    const nAt = (px) => 1 + ((px - m.l) / plotW) * (last - 1);
    const y = (s) => m.t + ((1 - s) / (1 - LO)) * plotH;

    for (let k = 0; k <= 5; k++) {
      const s = LO + (k * (1 - LO)) / 5;
      el('line', { x1: m.l, x2: m.l + plotW, y1: y(s), y2: y(s), class: k === 0 ? 'base' : 'grid' }, svg);
      const t = el('text', { x: m.l - 8, y: y(s) + 4, 'text-anchor': 'end', class: 'tick' }, svg);
      t.textContent = pct(s, 0);
    }

    // Per pixel column, the lowest and highest share of its games.
    const columns = [];
    let i = 0;
    for (let px = 0; px < plotW; px++) {
      let lo = Infinity;
      let hi = -Infinity;
      while (i < pts.length && x(pts[i].n) < m.l + px + 1) {
        lo = Math.min(lo, pts[i].share);
        hi = Math.max(hi, pts[i].share);
        i++;
      }
      if (lo !== Infinity) columns.push({ px: m.l + px + 0.5, lo, hi });
    }
    let band = '';
    for (const c of columns) {
      const top = y(c.hi);
      const bottom = Math.max(top + 1.6, y(c.lo));
      band += `M${c.px} ${top.toFixed(1)}V${bottom.toFixed(1)}`;
    }
    el('path', { d: band, 'stroke-width': 1.3, fill: 'none', style: 'stroke: var(--ink-2)' }, svg);

    // The last game's share, at its end.
    const endY = y(facts.last.share);
    el('circle', { cx: x(facts.last.n), cy: endY, r: 4, 'stroke-width': 2, style: 'fill: var(--ink-2); stroke: var(--paper)' }, svg);
    const endLabel = el('text', { x: x(facts.last.n) - 8, y: endY + 20, 'text-anchor': 'end', class: 'row-label' }, svg);
    endLabel.textContent = `${pct(facts.last.share)} in game ${fmt(facts.last.n)}`;

    // The n axis.
    const axisY = y(LO);
    const step = plotW < 420 ? 2000 : 1000;
    for (let n = 0; n <= last; n += step) {
      const nn = Math.max(1, n);
      el('line', { x1: x(nn), x2: x(nn), y1: axisY, y2: axisY + 5, class: 'base' }, svg);
      const t = el('text', { x: x(nn), y: axisY + 18, 'text-anchor': 'middle', class: 'tick' }, svg);
      t.textContent = n === 0 ? '1' : fmt(n);
    }

    // The hover layer: a crosshair that finds the searched game nearest the pointer.
    const cross = el('line', { y1: m.t, y2: axisY, class: 'cross', opacity: 0 }, svg);
    const dot = el('circle', { r: 4, 'stroke-width': 2, opacity: 0, style: 'fill: var(--ink); stroke: var(--paper)' }, svg);
    const hit = el('rect', { x: m.l, y: m.t, width: plotW, height: plotH, fill: 'transparent', class: 'hit', 'aria-label': 'Explore the searched games with the arrow keys' }, svg);
    const nearest = (n) => {
      let lo = 0;
      let hi = pts.length - 1;
      while (hi - lo > 1) {
        const mid = (lo + hi) >> 1;
        if (pts[mid].n < n) lo = mid;
        else hi = mid;
      }
      return Math.abs(pts[lo].n - n) <= Math.abs(pts[hi].n - n) ? lo : hi;
    };
    const showGame = (j) => {
      const p = pts[j];
      const px = x(p.n);
      cross.setAttribute('x1', px);
      cross.setAttribute('x2', px);
      cross.setAttribute('opacity', 1);
      dot.setAttribute('cx', px);
      dot.setAttribute('cy', y(p.share));
      dot.setAttribute('opacity', 1);
      tip.show([`Game ${fmt(p.n)}`, ['opt(n)', fmt(p.score)], ['above n/2', fmt(p.top)], ['share', pct(p.share, 2)]], px, m.t + 30);
    };
    const hide = () => {
      cross.setAttribute('opacity', 0);
      dot.setAttribute('opacity', 0);
      tip.hide();
    };
    hit.addEventListener('pointermove', (e) => {
      const r = svg.getBoundingClientRect();
      showGame(nearest(nAt(e.clientX - r.left)));
    });
    hit.addEventListener('pointerleave', hide);
    keyboard(hit, { count: pts.length, onIndex: showGame, onLeave: hide });
  });

  // The table view: the share by stretches of 500 games.
  const rows = [];
  for (let a = 1; a <= last; a += 500) {
    const b = Math.min(last, a + 499);
    const shares = pts.filter((p) => p.n >= a && p.n <= b).map((p) => p.share).sort((p, q) => p - q);
    if (!shares.length) continue;
    rows.push([`games ${fmt(a)} to ${fmt(b)}`, fmt(shares.length), pct(shares[0]), pct(shares[Math.floor(shares.length / 2)]), pct(shares[shares.length - 1])]);
  }
  tableView(host, 'The numbers behind the chart', ['stretch', 'searched', 'lowest share', 'median', 'highest'], rows);
  return facts;
}
