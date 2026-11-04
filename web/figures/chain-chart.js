// The whole chain, n = 1 to the last game, from data/games.json: the player's share of the board, opt(n) / (1 + 2 + ...
// + n); under it how each game was proven, in the three groups of js/proof.js (search, repeats an earlier search,
// derived from n − 1); and under those the time of the searches, the time of each search on a log scale and the
// running sum of those times in days.  All rows share the n axis, and the hover crosshair runs through all of them.
//
// The share is drawn as the range of its values within each pixel's games, so the zigzag between neighboring games
// shows as a band that narrows.  Each proof row is a small histogram: the share of each stretch's games proven so.
// A search is a game whose method is search (the chain's own searches, those that ended at the ceiling included); its
// time is the game's seconds over every run of the search (tools/export.mjs).

import { duration, el, fmt } from '../js/vocab.js';
import { GROUPS, ruleWords, settled } from '../js/proof.js';
import { freshSVG, keyboard, pct, responsive, tableView, tooltip } from '../js/chart.js';

const DAY = 86400;
// The scale of the time of a search: gridlines and labels at these.
const TIME_TICKS = [[1e-3, '1 ms'], [1, '1 s'], [60, '1 min'], [3600, '1 h'], [DAY, '1 day']];

export function mountChainChart(host, { games }) {
  const c = games.columns;
  const last = games.lastN;
  const count = c.n.length;
  const share = c.n.map((n, i) => c.score[i] / ((n * (n + 1)) / 2));
  const { groups, groupCounts } = settled(games);

  // The searches: each game's time if it ran a search of its own, and the time of the searches added up to each game.
  const searched = c.method.map((m) => m === 'search');
  const times = c.seconds.filter((_, i) => searched[i]);
  const searches = times.length;
  let running = 0;
  const added = c.seconds.map((s, i) => (running += searched[i] ? s : 0));
  const addedEnd = added[count - 1];
  const logLo = 10 ** Math.floor(Math.log10(Math.min(...times)));
  const logHi = 10 ** (Math.ceil(Math.log10(Math.max(...times)) * 4) / 4);
  const sumHi = 10 * Math.ceil(addedEnd / DAY / 10 + 0.5) * DAY;

  const box = host.querySelector('.chart-box');
  const tip = tooltip(box);
  const endShare = share[count - 1];
  host.querySelector('.chart-sub').textContent = `For every game from n = 1 to ${fmt(last)}. The player’s share of the board settles near ${pct(endShare)}; game ${fmt(last)} scores ${fmt(c.score[count - 1])} of ${fmt((last * (last + 1)) / 2)}.`;

  responsive(box, (width) => {
    const narrow = width < 620;
    // On a wide screen the names sit in a column to the left, each row's count beside it; on a narrow one, above.
    const labelW = narrow ? 0 : 238;
    const m = { l: narrow ? 46 : labelW + 14, r: narrow ? 10 : 18, t: 14 };
    const nameX = labelW - 52;
    const countX = m.l - 8;
    const shareH = narrow ? 140 : 170;
    const rowH = narrow ? 38 : 27;
    const logH = narrow ? 120 : 150;
    const sumH = narrow ? 64 : 80;
    const nameH = narrow ? 18 : 0; // a narrow row's name stands above its plot
    const top2 = m.t + shareH + 44;
    const top3 = top2 + GROUPS.length * rowH + 26;
    const logTop = top3 + nameH;
    const top4 = logTop + logH + 14;
    const sumTop = top4 + nameH;
    const axisY = sumTop + sumH + 4;
    const height = axisY + 34;
    const plotW = width - m.l - m.r;
    const svg = freshSVG(box, width, height, `Chart of the chain from n = 1 to ${last}: the player's share of the board, how each game was proven in three groups, the time of each of its ${fmt(searches)} searches, and those times added up, to ${(addedEnd / DAY).toFixed(1)} days. The numbers are in the table below.`);
    const x = (n) => m.l + ((n - 1) / (last - 1)) * plotW;
    const nAt = (px) => Math.min(last, Math.max(1, Math.round(1 + ((px - m.l) / plotW) * (last - 1))));
    const lo = 0.5;
    const hi = 0.75;
    const y = (s) => m.t + (1 - (s - lo) / (hi - lo)) * shareH;
    const logSpan = Math.log10(logHi / logLo);
    const yTime = (s) => logTop + (1 - Math.log10(s / logLo) / logSpan) * logH;
    const ySum = (s) => sumTop + (1 - s / sumHi) * sumH;

    // The name of a row at its left (a narrow screen: above), with a line under it.
    const rowName = (text, sub, ry, rh) => {
      const name = el('text', { x: narrow ? m.l : nameX, y: narrow ? ry - 6 : ry + rh / 2 - (sub ? 4 : -4), 'text-anchor': narrow ? 'start' : 'end', class: 'row-label' }, svg);
      name.textContent = text;
      if (sub) {
        const s = el('text', { x: narrow ? m.l + plotW : nameX, y: narrow ? ry - 6 : ry + rh / 2 + 13, 'text-anchor': 'end', class: 'row-count' }, svg);
        s.textContent = sub;
      }
    };

    // The share: gridlines, then one band of the values each pixel covers.
    const g = el('g', {}, svg);
    for (let s = lo; s <= hi + 1e-9; s += 0.05) {
      el('line', { x1: m.l, x2: m.l + plotW, y1: y(s), y2: y(s), class: 'grid' }, g);
      const t = el('text', { x: m.l - 8, y: y(s) + 4, 'text-anchor': 'end', class: 'tick' }, g);
      t.textContent = pct(s, 0);
    }
    if (!narrow) rowName('player’s share', 'opt(n) / (1 + 2 + ... + n)', m.t, shareH);
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

    // How each game was proven: a row a group, a bar a stretch of games, its height the stretch's share.
    const binPx = 6;
    const bins = Math.max(10, Math.floor(plotW / binPx));
    const per = count / bins;
    GROUPS.forEach((grp, r) => {
      const ry = top2 + r * rowH;
      const base = ry + rowH - 4;
      const barH = rowH - (narrow ? 20 : 8);
      el('line', { x1: m.l, x2: m.l + plotW, y1: base + 0.5, y2: base + 0.5, class: 'base' }, svg);
      const name = el('text', { x: narrow ? m.l : nameX, y: narrow ? ry + 11 : base - 4, 'text-anchor': narrow ? 'start' : 'end', class: 'row-label' }, svg);
      name.textContent = grp.name;
      const k = el('text', { x: narrow ? m.l + plotW : countX, y: narrow ? ry + 11 : base - 4, 'text-anchor': 'end', class: 'row-count' }, svg);
      k.textContent = fmt(groupCounts[grp.key]);
      let path = '';
      for (let b = 0; b < bins; b++) {
        const i0 = Math.floor(b * per);
        const i1 = Math.floor((b + 1) * per);
        let hits = 0;
        for (let j = i0; j < i1; j++) if (groups[j] === grp.key) hits++;
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
    } else {
      const head = el('text', { x: m.l, y: top2 - 8, class: 'row-count' }, svg);
      head.textContent = 'how each game was proven';
    }

    // The time of each search: a dot a search, its seconds on a log scale.
    rowName('time of each search', `${fmt(searches)} searches, log scale`, logTop, logH);
    for (const [s, label] of TIME_TICKS) {
      el('line', { x1: m.l, x2: m.l + plotW, y1: yTime(s), y2: yTime(s), class: 'grid' }, svg);
      const t = el('text', { x: m.l - 8, y: yTime(s) + 4, 'text-anchor': 'end', class: 'tick' }, svg);
      t.textContent = label;
    }
    el('line', { x1: m.l, x2: m.l + plotW, y1: logTop + logH + 0.5, y2: logTop + logH + 0.5, class: 'base' }, svg);
    const dots = el('g', { style: 'fill: var(--ink-2)', 'fill-opacity': 0.35 }, svg);
    c.n.forEach((n, j) => {
      if (searched[j]) el('circle', { cx: x(n).toFixed(1), cy: yTime(c.seconds[j]).toFixed(1), r: 1.3 }, dots);
    });

    // The search time added up: a line on a linear scale in days.
    rowName('search time added up', 'in days', sumTop, sumH);
    for (let days = 0; days * DAY <= sumHi + 1e-6; days += 10) {
      el('line', { x1: m.l, x2: m.l + plotW, y1: ySum(days * DAY), y2: ySum(days * DAY), class: days ? 'grid' : 'base' }, svg);
      const t = el('text', { x: m.l - 8, y: ySum(days * DAY) + 4, 'text-anchor': 'end', class: 'tick' }, svg);
      t.textContent = days ? `${days} d` : '0';
    }
    el('path', { d: `M${c.n.map((n, j) => `${x(n).toFixed(1)} ${ySum(added[j]).toFixed(1)}`).join('L')}`, fill: 'none', 'stroke-width': 1.6, style: 'stroke: var(--ink)' }, svg);
    el('circle', { cx: x(last), cy: ySum(addedEnd), r: 3.5, 'stroke-width': 2, style: 'fill: var(--ink); stroke: var(--paper)' }, svg);
    const sumLabel = el('text', { x: x(last) - 8, y: ySum(addedEnd) - 9, 'text-anchor': 'end', class: 'row-label' }, svg);
    sumLabel.textContent = `${(addedEnd / DAY).toFixed(1)} days at n = ${fmt(last)}`;

    // The n axis, shared by all.
    const step = plotW < 420 ? 2000 : 1000;
    for (let n = 0; n <= last; n += step) {
      const nn = Math.max(1, n);
      el('line', { x1: x(nn), x2: x(nn), y1: axisY, y2: axisY + 5, class: 'base' }, svg);
      const t = el('text', { x: x(nn), y: axisY + 18, 'text-anchor': 'middle', class: 'tick' }, svg);
      t.textContent = n === 0 ? '1' : fmt(n);
    }
    el('line', { x1: m.l, x2: m.l + plotW, y1: y(lo) + 0.5, y2: y(lo) + 0.5, class: 'base' }, svg);

    // The hover layer: a crosshair that finds the game under the pointer, with a mark on each row that has one.
    const cross = el('line', { y1: m.t, y2: axisY, class: 'cross', opacity: 0 }, svg);
    const dot = el('circle', { r: 4, 'stroke-width': 2, opacity: 0, style: 'fill: var(--player); stroke: var(--paper)' }, svg);
    const timeDot = el('circle', { r: 3.2, 'stroke-width': 1.5, opacity: 0, style: 'fill: none; stroke: var(--ink)' }, svg);
    const sumDot = el('circle', { r: 3.2, 'stroke-width': 2, opacity: 0, style: 'fill: var(--ink); stroke: var(--paper)' }, svg);
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
      sumDot.setAttribute('cx', px);
      sumDot.setAttribute('cy', ySum(added[j]));
      sumDot.setAttribute('opacity', 1);
      timeDot.setAttribute('cx', px);
      if (searched[j]) timeDot.setAttribute('cy', yTime(c.seconds[j]));
      timeDot.setAttribute('opacity', searched[j] ? 1 : 0);
      const [group, how] = ruleWords(c, j);
      const rows = [`Game ${fmt(n)}`, ['opt(n)', fmt(c.score[j])], ['player’s share', pct(share[j], 2)], ['proven', group], ['by', how]];
      if (searched[j]) rows.push(['time of its search', duration(c.seconds[j])]);
      rows.push(['search time added up', duration(added[j])]);
      tip.show(rows, px, m.t + 40);
    };
    const hide = () => {
      cross.setAttribute('opacity', 0);
      dot.setAttribute('opacity', 0);
      timeDot.setAttribute('opacity', 0);
      sumDot.setAttribute('opacity', 0);
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
    const part = groups.slice(a - 1, b);
    const byGroup = GROUPS.map((grp) => fmt(part.filter((k) => k === grp.key).length));
    const mine = c.seconds.slice(a - 1, b).filter((_, j) => searched[a - 1 + j]);
    const sh = share.slice(Math.max(1, a - 1), b);
    rows.push([`${fmt(a)} to ${fmt(b)}`, ...byGroup, fmt(mine.length), duration(mine.reduce((s, t) => s + t, 0)), `${pct(Math.min(...sh))} to ${pct(Math.max(...sh))}`]);
  }
  rows.push(['all', ...GROUPS.map((grp) => fmt(groupCounts[grp.key])), fmt(searches), duration(addedEnd), '']);
  tableView(host, 'The numbers behind the chart', ['games', ...GROUPS.map((grp) => grp.name), 'searches', 'search time added up', 'player’s share'], rows);
}
