// How every game of the chain was settled, in the terms of chapter 2: a cell for each game, a hundred to a row, game 1
// at the top left, so that the grid grows downward as the chain does.  Each cell is
// shaded by the rule of its game (js/proof.js), from data/games.json alone:
//
//   the ceiling        a game whose score is n + opt(n − 1), found with no search
//   the prime formula  a prime n
//   a repeat           a game that repeats an earlier game's search
//   a search           every other game: a search of its own, or nothing to promote
//
// A game that the ceiling or the prime formula settles is counted there whatever its record says, since both prove it
// from the game before; the record is in its tooltip.  The two rules that need a search wear the search's green, a
// search dark and a repeat light, and the two that need none are gray (css/site.css).

import { el, fmt } from '../js/vocab.js';
import { freshSVG, keyboard, pct, responsive, tableView, tooltip } from '../js/chart.js';
import { NO_SEARCH, RULE, RULES, recordWords, settled } from '../js/proof.js';

/** A legend swatch for a rule, as HTML. */
export function swatch(key) {
  return `<span class="rule-swatch is-${key}" aria-hidden="true"></span>`;
}

/** The legend of the rules that the predicate used(key) keeps, in the order of RULES, with each one's count if counts
 *  is given, as HTML. */
export function legendHTML(used, counts = null) {
  return RULES.filter((r) => used(r.key))
    .map((r) => `<span class="key-item">${swatch(r.key)}${r.name}${counts ? `<b>${fmt(counts[r.key])}</b>` : ''}</span>`)
    .join('');
}

/** Draw one cell of the rule key at (x, y), w wide and h high, into parent. */
export function ruleCell(parent, key, x, y, w, h = w) {
  return el('rect', { x, y, width: w, height: h, rx: Math.min(2, w / 4, h / 4), class: `rule-cell is-${key}` }, parent);
}

export function mountSettled(host, { games }) {
  const c = games.columns;
  const facts = settled(games);
  const { rules, counts, total } = facts;
  const last = games.lastN;
  const box = host.querySelector('.chart-box');
  const tip = tooltip(box);

  const legend = host.querySelector('.rule-legend');
  legend.innerHTML = legendHTML((key) => counts[key] > 0, counts);

  responsive(box, (width) => {
    const labelW = 46;
    const cols = 100;
    const pitch = (width - labelW - 2) / cols;
    const gap = pitch >= 7 ? 2 : 1;
    const size = pitch - gap;
    const rows = Math.ceil(total / cols);
    const top = 6;
    const height = top + rows * pitch + 8;
    const svg = freshSVG(box, width, height, `Every game of the chain from game 1 to game ${fmt(last)}, a cell each, `
      + `${cols} to a row from game 1 at the top left, colored by how it was settled. `
      + 'The counts are in the legend and in the table below.');
    const at = (i) => {
      const r = Math.floor(i / cols);
      return { x: labelW + (i % cols) * pitch, y: top + r * pitch };
    };
    const g = el('g', {}, svg);
    rules.forEach((key, i) => {
      const p = at(i);
      ruleCell(g, key, p.x, p.y, size);
    });
    // The first game of every thousandth row at its left, and the last game after its cell.
    const every = 10;
    for (let r = 0; r < rows; r += every) {
      const t = el('text', { x: labelW - 8, y: at(r * cols).y + size / 2 + 4, 'text-anchor': 'end', class: 'tick' }, svg);
      t.textContent = fmt(r * cols + 1);
    }
    const lastAt = at(total - 1);
    const lastLabel = el('text', { x: lastAt.x + size + 6, y: lastAt.y + size / 2 + 4, class: 'tick' }, svg);
    lastLabel.textContent = fmt(last);

    // The hover layer: the cell under the pointer, and its game in the tooltip.
    const ring = el('rect', { width: size + 3, height: size + 3, rx: 2, class: 'rule-ring', opacity: 0 }, svg);
    const hit = el('rect', {
      x: labelW, y: top, width: cols * pitch, height: rows * pitch, fill: 'transparent', class: 'hit',
      'aria-label': 'Explore the games with the arrow keys',
    }, svg);
    const show = (i) => {
      const p = at(i);
      ring.setAttribute('x', p.x - 1.5);
      ring.setAttribute('y', p.y - 1.5);
      ring.setAttribute('opacity', 1);
      const key = rules[i];
      const lines = [`Game ${fmt(c.n[i])}`, ['opt(n)', fmt(c.score[i])], ['settled by', RULE[key].name]];
      if (c.proof[i] !== RULE[key].proof) lines.push(['the chain’s proof', recordWords(c, i)]);
      if (c.proof[i] === 'exhausted') lines.push(['nodes', fmt(c.nodes[i])]);
      tip.show(lines, p.x + size / 2, p.y);
    };
    const hide = () => {
      ring.setAttribute('opacity', 0);
      tip.hide();
    };
    hit.addEventListener('pointermove', (e) => {
      const r = svg.getBoundingClientRect();
      const col = Math.floor((e.clientX - r.left - labelW) / pitch);
      const row = Math.floor((e.clientY - r.top - top) / pitch);
      const i = row * cols + col;
      if (col < 0 || col >= cols || row < 0 || i >= total) hide();
      else show(i);
    });
    hit.addEventListener('pointerleave', hide);
    keyboard(hit, { count: total, onIndex: show, onLeave: hide });
  });

  const rows = [];
  for (let a = 1; a <= last; a += 500) {
    const b = Math.min(last, a + 499);
    const part = rules.slice(a - 1, b);
    const noSearch = part.filter((k) => NO_SEARCH.includes(k)).length;
    const each = RULES.map((r) => fmt(part.filter((k) => k === r.key).length));
    rows.push([`games ${fmt(a)} to ${fmt(b)}`, ...each, pct(noSearch / part.length, 0)]);
  }
  rows.push(['all', ...RULES.map((r) => fmt(counts[r.key])), pct(NO_SEARCH.reduce((s, k) => s + counts[k], 0) / total, 0)]);
  tableView(host, 'The numbers behind the chart', ['stretch', ...RULES.map((r) => r.name), 'no search'], rows);
  return facts;
}
