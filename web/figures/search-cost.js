// The cost of the search: the nodes of every game of the chain that needed one, against n, on a log scale, all from
// data/games.json (the games whose method is "search").  One series, so no legend: a dot a game, in ink, with the
// last game labeled; a crosshair and a tooltip find the game under the pointer or the arrow keys, and a table gives
// the numbers by stretches of 500 games.

import { el, fmt } from '../js/vocab.js';
import { freshSVG, keyboard, responsive, tableView, tooltip } from '../js/chart.js';

const PROOF_WORDS = { exhausted: 'a search of the whole tree', cap: 'the ceiling, which the search reached' };

/** The searched games: their n, nodes and proof, in order of n. */
export function searchedGames(games) {
  const c = games.columns;
  const out = [];
  for (let i = 0; i < c.n.length; i++) {
    if (c.method[i] === 'search') out.push({ n: c.n[i], nodes: c.nodes[i], proof: c.proof[i] });
  }
  return out;
}

const median = (xs) => {
  const s = [...xs].sort((a, b) => a - b);
  const m = s.length >> 1;
  return s.length % 2 ? s[m] : Math.round((s[m - 1] + s[m]) / 2);
};

/** A number of nodes in words: 3179486263 as "3.2 billion". */
export function nodesWords(x) {
  if (x >= 1e9) return `${(x / 1e9).toFixed(1)} billion`;
  if (x >= 1e6) return `${(x / 1e6).toFixed(x >= 1e8 ? 0 : 1)} million`;
  return fmt(x);
}

export function mountSearchCost(host, { games }) {
  const list = searchedGames(games);
  const last = games.lastN;
  const box = host.querySelector('.chart-box');
  const tip = tooltip(box);
  const end = list[list.length - 1];
  host.querySelector('.chart-sub').textContent = `The ${fmt(list.length)} games of the chain from n = 1 to ${fmt(last)} that needed a search, one dot a game. Each grid line is ten times the one below it.`;
  const top = Math.ceil(Math.log10(Math.max(...list.map((g) => g.nodes))));

  responsive(box, (width) => {
    const narrow = width < 520;
    const m = { l: narrow ? 50 : 76, r: narrow ? 12 : 22, t: 24, b: 34 };
    const plotH = narrow ? 230 : 290;
    const height = m.t + plotH + m.b;
    const plotW = width - m.l - m.r;
    const svg = freshSVG(box, width, height, `Chart of the nodes of each searched game of the chain against n, on a log scale, from 1 node to ${nodesWords(10 ** top)}. The numbers are in the table below.`);
    const x = (n) => m.l + ((n - 1) / (last - 1)) * plotW;
    const y = (nodes) => m.t + (1 - Math.log10(Math.max(1, nodes)) / top) * plotH;

    // Grid lines a decade apart, labeled every third, in words.
    const g = el('g', {}, svg);
    for (let d = 0; d <= top; d++) {
      el('line', { x1: m.l, x2: m.l + plotW, y1: y(10 ** d), y2: y(10 ** d), class: d === 0 ? 'base' : 'grid' }, g);
      if (d % 3 !== 0) continue;
      const t = el('text', { x: m.l - 8, y: y(10 ** d) + 4, 'text-anchor': 'end', class: 'tick' }, g);
      t.textContent = { 0: '1', 3: '1,000', 6: narrow ? '10⁶' : '1 million', 9: narrow ? '10⁹' : '1 billion' }[d] ?? `10^${d}`;
    }
    const yLabel = el('text', { x: m.l - 8, y: m.t - 10, 'text-anchor': 'end', class: 'row-count' }, g);
    yLabel.textContent = 'nodes';

    // The dots.
    let d = '';
    for (const game of list) d += `M${x(game.n).toFixed(1)} ${y(game.nodes).toFixed(1)}h0`;
    el('path', { d, class: 'cost-dots', 'stroke-width': narrow ? 3 : 3.4, 'stroke-linecap': 'round', fill: 'none' }, svg);
    // The last game, labeled below the cloud of the late games, with a leader up to its dot.
    // On a phone the prose beside the chart names it instead.
    el('circle', { cx: x(end.n), cy: y(end.nodes), r: 4.5, 'stroke-width': 2, class: 'cost-end' }, svg);
    if (!narrow) {
      const labelY = y(10 ** (top * 0.55));
      el('line', { x1: x(end.n), x2: x(end.n), y1: y(end.nodes) + 6, y2: labelY - 4, class: 'cross' }, svg);
      const endLabel = el('text', { x: x(end.n) - 4, y: labelY + 10, 'text-anchor': 'end', class: 'row-label' }, svg);
      endLabel.textContent = `game ${fmt(end.n)}: ${nodesWords(end.nodes)}`;
    }

    // The n axis.
    const step = plotW < 420 ? 2000 : 1000;
    const axisY = m.t + plotH;
    for (let n = 0; n <= last; n += step) {
      const nn = Math.max(1, n);
      el('line', { x1: x(nn), x2: x(nn), y1: axisY, y2: axisY + 5, class: 'base' }, svg);
      const t = el('text', { x: x(nn), y: axisY + 18, 'text-anchor': 'middle', class: 'tick' }, svg);
      t.textContent = n === 0 ? '1' : fmt(n);
    }
    const nLabel = el('text', { x: m.l + plotW, y: axisY + 32, 'text-anchor': 'end', class: 'row-count' }, svg);
    nLabel.textContent = 'n';

    // The hover layer: the searched game nearest the pointer.
    const cross = el('line', { y1: m.t, y2: axisY, class: 'cross', opacity: 0 }, svg);
    const dot = el('circle', { r: 4.5, 'stroke-width': 2, opacity: 0, class: 'cost-end' }, svg);
    const hit = el('rect', { x: m.l, y: m.t, width: plotW, height: plotH, fill: 'transparent', class: 'hit', 'aria-label': 'Explore the searched games with the arrow keys' }, svg);
    const show = (j) => {
      const game = list[j];
      const px = x(game.n);
      cross.setAttribute('x1', px);
      cross.setAttribute('x2', px);
      cross.setAttribute('opacity', 1);
      dot.setAttribute('cx', px);
      dot.setAttribute('cy', y(game.nodes));
      dot.setAttribute('opacity', 1);
      tip.show([`Game ${fmt(game.n)}`, ['nodes', fmt(game.nodes)], ['proven by', PROOF_WORDS[game.proof] ?? game.proof]], px, m.t + 40);
    };
    const hide = () => {
      cross.setAttribute('opacity', 0);
      dot.setAttribute('opacity', 0);
      tip.hide();
    };
    const nearest = (n) => {
      let lo = 0;
      let hi = list.length - 1;
      while (hi - lo > 1) {
        const mid = (lo + hi) >> 1;
        if (list[mid].n <= n) lo = mid;
        else hi = mid;
      }
      return Math.abs(list[lo].n - n) <= Math.abs(list[hi].n - n) ? lo : hi;
    };
    hit.addEventListener('pointermove', (e) => {
      const r = svg.getBoundingClientRect();
      show(nearest(1 + ((e.clientX - r.left - m.l) / plotW) * (last - 1)));
    });
    hit.addEventListener('pointerleave', hide);
    keyboard(hit, { count: list.length, onIndex: show, onLeave: hide });
  });

  // The table: by stretches of 500 games, the searched games, and their nodes.
  const rows = [];
  for (let a = 1; a <= last; a += 500) {
    const b = Math.min(last, a + 499);
    const part = list.filter((g) => g.n >= a && g.n <= b);
    if (!part.length) continue;
    const most = part.reduce((p, q) => (q.nodes > p.nodes ? q : p));
    rows.push([`${fmt(a)} to ${fmt(b)}`, fmt(part.length), fmt(median(part.map((g) => g.nodes))), `${fmt(most.nodes)} (game ${fmt(most.n)})`]);
  }
  tableView(host, 'The numbers behind the chart', ['games', 'searched', 'median nodes', 'most nodes'], rows);
  return { count: list.length, end, median };
}
