// The whole chain, n = 1 to the last game, from data/games.json: the player's share of the board, opt(n) / (1 + 2 + ...
// + n); under it how each game was proven and the time of a search, each as one column for every stretch of 100 games.
// All rows share the n axis, and the hover crosshair runs through all of them.
//
// The share is drawn as the range of its values within each pixel's games, so the zigzag between neighboring games
// shows as a band that narrows.  The proof row stacks the stretch's games by the three groups of js/proof.js (search,
// repeats an earlier search, derived from n − 1) on a 0 to 100% scale.  The time row rises on a log scale to the
// median of the stretch's searches.  A search is a game whose method is search (the chain's own searches, those that
// ended at the ceiling included); its time is the game's seconds over every run of the search (tools/export.mjs).

import { duration, el, fmt } from '../js/vocab.js';
import { GROUPS, ruleWords, settled } from '../js/proof.js';
import { freshSVG, keyboard, pct, responsive, tableView, tooltip } from '../js/chart.js';

const DAY = 86400;
const STRETCH = 100;
// The scale of the time of a search: gridlines and labels at these.
const TIME_TICKS = [[1e-3, '1 ms'], [1, '1 s'], [60, '1 min'], [3600, '1 h'], [DAY, '1 day']];
const TIME_LO = 1e-3;
const TIME_HI = 1.6 * DAY;
// A group's tone: the gray ramp of chapter 2's grid (css/site.css), the search darkest.
const TONE = { search: 'var(--rule-search)', repeat: 'var(--rule-repeat)', before: 'var(--rule-ceiling)' };

const median = (xs) => {
  if (!xs.length) return null;
  const s = [...xs].sort((p, q) => p - q);
  const h = s.length >> 1;
  return s.length % 2 ? s[h] : (s[h - 1] + s[h]) / 2;
};

export function mountChainChart(host, { games }) {
  const c = games.columns;
  const last = games.lastN;
  const count = c.n.length;
  const share = c.n.map((n, i) => c.score[i] / ((n * (n + 1)) / 2));
  const { groups, groupCounts } = settled(games);
  const searched = c.method.map((m) => m === 'search');
  const searches = searched.filter(Boolean).length;

  // The stretches of 100 games (the last is shorter): how many games of each group, and the searches among them.
  const stretches = [];
  for (let a = 1; a <= last; a += STRETCH) {
    const b = Math.min(last, a + STRETCH - 1);
    const inStretch = (_, j) => j >= a - 1 && j < b;
    const part = groups.filter(inStretch);
    const by = Object.fromEntries(GROUPS.map((grp) => [grp.key, part.filter((k) => k === grp.key).length]));
    const times = c.seconds.filter((_, j) => inStretch(_, j) && searched[j]);
    const sh = share.slice(Math.max(1, a - 1), b);
    stretches.push({
      a, b, games: b - a + 1, by, searches: times.length, median: median(times),
      lo: Math.min(...sh), hi: Math.max(...sh),
    });
  }

  const box = host.querySelector('.chart-box');
  const tip = tooltip(box);
  const endShare = share[count - 1];
  host.querySelector('.chart-sub').textContent =
    `For games 1 to ${fmt(last)}, in stretches of ${STRETCH} games. The player’s share of the board ` +
    `settles near ${pct(endShare)}; game ${fmt(last)} scores ${fmt(c.score[count - 1])} of ` +
    `${fmt((last * (last + 1)) / 2)}. Under it, how the games of each stretch were proven, and the median time ` +
    'of its searches.';

  responsive(box, (width) => {
    const narrow = width < 620;
    // On a wide screen the names sit in a column to the left; on a narrow one, above their rows.
    const labelW = narrow ? 0 : 238;
    const m = { l: narrow ? 46 : labelW + 14, r: narrow ? 10 : 18, t: 14 };
    const nameX = labelW - 52;
    const shareH = narrow ? 140 : 170;
    const provH = narrow ? 90 : 100;
    const timeH = narrow ? 110 : 130;
    const nameH = narrow ? 18 : 0; // a narrow row's name stands above its plot
    const keyH = narrow ? 52 : 0; // a narrow screen's key stands above the proof row's plot
    const provTop = m.t + shareH + 40;
    const provPlot = provTop + nameH + keyH;
    const timeTop = provPlot + provH + 30;
    const timePlot = timeTop + nameH;
    const axisY = timePlot + timeH + 4;
    const height = axisY + 34;
    const plotW = width - m.l - m.r;
    const svg = freshSVG(box, width, height,
      `Chart of the chain from game 1 to game ${fmt(last)}: the player's share of the board; for each stretch of ` +
      `${STRETCH} games, ` +
      'how its games were proven in three groups and the median time of its searches. ' +
      'The numbers are in the table below.');
    const x = (n) => m.l + ((n - 1) / (last - 1)) * plotW;
    const nAt = (px) => Math.min(last, Math.max(1, Math.round(1 + ((px - m.l) / plotW) * (last - 1))));
    const lo = 0.5;
    const hi = 0.75;
    const y = (s) => m.t + (1 - (s - lo) / (hi - lo)) * shareH;
    const yProv = (f) => provPlot + (1 - f) * provH;
    const logSpan = Math.log10(TIME_HI / TIME_LO);
    const yTime = (s) => timePlot + (1 - Math.log10(s / TIME_LO) / logSpan) * timeH;
    // A stretch's column spans its games on the n axis, with a hairline gap between neighbors.
    const colX = (st) => x(st.a) + 0.5;
    const colW = (st) => Math.max(1, x(st.b === last ? last : st.b + 1) - x(st.a) - 1);

    // The name of a row at its left (a narrow screen: above), with a line under it.
    const rowName = (text, sub, ry, rh) => {
      const name = el('text', {
        x: narrow ? m.l : nameX, y: narrow ? ry - 6 : ry + rh / 2 - (sub ? 4 : -4),
        'text-anchor': narrow ? 'start' : 'end', class: 'row-label',
      }, svg);
      name.textContent = text;
      if (sub) {
        const s = el('text', {
          x: narrow ? m.l + plotW : nameX, y: narrow ? ry - 6 : ry + rh / 2 + 13,
          'text-anchor': 'end', class: 'row-count',
        }, svg);
        s.textContent = sub;
      }
    };
    const gridRow = (ticks, yOf) => {
      for (const [v, label] of ticks) {
        el('line', { x1: m.l, x2: m.l + plotW, y1: yOf(v), y2: yOf(v), class: 'grid' }, svg);
        const t = el('text', { x: m.l - 8, y: yOf(v) + 4, 'text-anchor': 'end', class: 'tick' }, svg);
        t.textContent = label;
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
    el('circle', {
      cx: x(last), cy: y(endShare), r: 4.5, 'stroke-width': 2, style: 'fill: var(--player); stroke: var(--paper)',
    }, g);
    const endLabel = el('text', { x: x(last) - 8, y: y(endShare) - 12, 'text-anchor': 'end', class: 'row-label' }, g);
    endLabel.textContent = `${pct(endShare)} at game ${fmt(last)}`;

    // How each game was proven: a stacked column a stretch, from the bottom derived from n − 1, repeats, search.
    rowName('how each game was proven', `each ${STRETCH} games, whole = 100%`, provTop + nameH, 24);
    gridRow([[0, '0'], [0.5, '50%'], [1, '100%']], yProv);
    const stack = [...GROUPS].reverse();
    for (const st of stretches) {
      let from = 0;
      for (const grp of stack) {
        const f = st.by[grp.key] / st.games;
        if (!f) continue;
        el('rect', {
          x: colX(st), y: yProv(from + f), width: colW(st), height: f * provH, style: `fill: ${TONE[grp.key]}`,
        }, svg);
        from += f;
      }
    }
    el('line', { x1: m.l, x2: m.l + plotW, y1: yProv(0) + 0.5, y2: yProv(0) + 0.5, class: 'base' }, svg);
    // The key: the groups top to bottom as the columns stack them, each with its games over the whole chain.
    const keyY = narrow ? provTop + nameH + 8 : provTop + 48;
    GROUPS.forEach((grp, r) => {
      const ky = keyY + r * 17;
      el('rect', {
        x: narrow ? m.l : 4, y: ky - 10, width: 12, height: 12, rx: 2, style: `fill: ${TONE[grp.key]}`,
      }, svg);
      const k = el('text', { x: narrow ? m.l + 54 : 58, y: ky, 'text-anchor': 'end', class: 'row-count' }, svg);
      k.textContent = fmt(groupCounts[grp.key]);
      const name = el('text', { x: narrow ? m.l + 62 : 66, y: ky, class: 'row-count' }, svg);
      name.textContent = grp.name;
    });

    // The time of a search: a column a stretch, rising to the median of its searches on a log scale.
    rowName('time required to search', 'median per 100 games, log scale', timePlot, timeH);
    gridRow(TIME_TICKS, yTime);
    for (const st of stretches) {
      if (st.median === null) continue;
      const top = yTime(Math.min(TIME_HI, Math.max(TIME_LO, st.median)));
      el('rect', {
        x: colX(st), y: top, width: colW(st), height: yTime(TIME_LO) - top, style: 'fill: var(--time)',
      }, svg);
    }
    const timeBase = timePlot + timeH + 0.5;
    el('line', { x1: m.l, x2: m.l + plotW, y1: timeBase, y2: timeBase, class: 'base' }, svg);

    // The n axis, shared by all.
    const step = plotW < 420 ? 2000 : 1000;
    for (let n = 0; n <= last; n += step) {
      const nn = Math.max(1, n);
      el('line', { x1: x(nn), x2: x(nn), y1: axisY, y2: axisY + 5, class: 'base' }, svg);
      const t = el('text', { x: x(nn), y: axisY + 18, 'text-anchor': 'middle', class: 'tick' }, svg);
      t.textContent = n === 0 ? '1' : fmt(n);
    }
    el('line', { x1: m.l, x2: m.l + plotW, y1: y(lo) + 0.5, y2: y(lo) + 0.5, class: 'base' }, svg);

    // The hover layer: a crosshair that finds the game under the pointer, with the share's dot.
    const cross = el('line', { y1: m.t, y2: axisY, class: 'cross', opacity: 0 }, svg);
    const dot = el('circle', {
      r: 4, 'stroke-width': 2, opacity: 0, style: 'fill: var(--player); stroke: var(--paper)',
    }, svg);
    const hit = el('rect', {
      x: m.l, y: m.t, width: plotW, height: axisY - m.t, fill: 'transparent', class: 'hit',
      'aria-label': 'Explore the chain with the arrow keys',
    }, svg);
    const showGame = (n) => {
      const j = n - 1;
      const px = x(n);
      const st = stretches[Math.floor(j / STRETCH)];
      cross.setAttribute('x1', px);
      cross.setAttribute('x2', px);
      cross.setAttribute('opacity', 1);
      dot.setAttribute('cx', px);
      dot.setAttribute('cy', y(Math.max(lo, Math.min(hi, share[j]))));
      dot.setAttribute('opacity', n > 1 ? 1 : 0);
      const rows = [
        `Game ${fmt(n)}`, ['opt(n)', fmt(c.score[j])], ['player’s share', pct(share[j], 2)],
        ['proven by', ruleWords(c, j)],
        `Games ${fmt(st.a)} to ${fmt(st.b)}`,
        ...GROUPS.map((grp) => [grp.name, fmt(st.by[grp.key])]),
        ['median search time', duration(st.median)],
      ];
      tip.show(rows, px, m.t + 40);
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

  // The table view: the numbers behind the chart, by stretches of 100 games.
  const rows = stretches.map((st) => [
    `games ${fmt(st.a)} to ${fmt(st.b)}`, ...GROUPS.map((grp) => fmt(st.by[grp.key])), fmt(st.searches), duration(st.median),
    `${pct(st.lo)} to ${pct(st.hi)}`,
  ]);
  const allSearch = c.seconds.filter((_, j) => searched[j]);
  rows.push(['all', ...GROUPS.map((grp) => fmt(groupCounts[grp.key])), fmt(searches), duration(median(allSearch)), '']);
  const head = ['stretch', ...GROUPS.map((grp) => grp.name), 'searches', 'median search time', 'player’s share'];
  tableView(host, 'The numbers behind the chart', head, rows);
}
