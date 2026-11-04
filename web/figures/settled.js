// How every game of the chain was settled, in the terms of chapter 2: a cell for each game, a hundred to a row, game 1
// at the bottom left, so that the chain climbs as the board of the walker does.  Each cell is shaded by the first of
// the chapter's rules that settles its game, from data/games.json alone:
//
//   the ceiling        a composite n whose score is n + opt(n − 1), found with no search
//   the prime formula  a prime n
//   a repeat           a game below its ceiling that the chain proves as a repeat of an earlier game's search
//   a search           a game whose search ran (method search)
// and two that no rule of the chapter settles: nothing to choose (proof trivial) and the bounds store (proof known).
// A game at its ceiling is counted there whatever its record says, since the ceiling lemma proves it from the game
// before; the record is in its tooltip.  The four rules take a gray ramp, light to dark in the order the chapter
// meets them, checked with the dataviz validator (--ordinal: monotone lightness, the light end at 2.2:1 on white).

import { primeFactors } from '../js/model.js';
import { el, fmt } from '../js/vocab.js';
import { freshSVG, keyboard, pct, responsive, tableView, tooltip } from '../js/chart.js';

export const isPrime = (n) => n >= 2 && primeFactors(n)[0] === n;

/** The chapter's rules, in the order it meets them, and the two kinds of game no rule of it settles.  Each is drawn
 *  by its class, rule-cell is-<key>, in css/chain.css: the rules a gray ramp, nothing to choose a hollow cell and the
 *  bounds store the palest gray.  proof is the record's proof kind that is the rule's own. */
export const RULES = [
  { key: 'ceiling', name: 'at the ceiling', proof: 'cap' },
  { key: 'prime', name: 'the prime formula', proof: 'prime_formula' },
  { key: 'repeat', name: 'a repeat', proof: 'same_search' },
  { key: 'search', name: 'a search', proof: 'exhausted' },
  { key: 'nothing', name: 'nothing to choose', proof: 'trivial' },
  { key: 'store', name: 'the bounds store', proof: 'known' },
];
export const RULE = Object.fromEntries(RULES.map((r) => [r.key, r]));
/** The rules that settle a game with no search. */
export const NO_SEARCH = ['ceiling', 'prime', 'repeat'];

/** The rule that settles the game in row i of games.json's columns c. */
export function ruleOf(c, i) {
  const n = c.n[i];
  if (isPrime(n)) return 'prime';
  if (c.method[i] === 'search') return 'search';
  if (i > 0 && c.score[i] === c.score[i - 1] + n) return 'ceiling';
  if (c.proof[i] === 'same_search') return 'repeat';
  if (c.proof[i] === 'known') return 'store';
  return 'nothing';
}

/** The game whose search a repeat n repeats: the game before it, or the one before that when n − 1 is prime, since a
 *  prime game runs no search (results/README.md, the paragraph on repeats, which every repeat of the chain keeps). */
export const repeatOf = (n) => (isPrime(n - 1) ? n - 2 : n - 1);

/** How the chain's record proves the game in row i, in words: "a repeat of game 21's search". */
export function recordWords(c, i) {
  const n = c.n[i];
  switch (c.proof[i]) {
    case 'prime_formula': return 'the prime formula';
    case 'cap': return 'the ceiling';
    case 'same_search': return `a repeat of game ${fmt(repeatOf(n))}’s search`;
    case 'exhausted': return `a search of ${fmt(c.nodes[i])} node${c.nodes[i] === 1 ? '' : 's'}`;
    case 'trivial': return 'nothing to choose';
    case 'known': return 'the bounds store';
    default: return c.proof[i];
  }
}

/** Every game's rule, and the count of each. */
export function settled(games) {
  const c = games.columns;
  const rules = c.n.map((_, i) => ruleOf(c, i));
  const counts = Object.fromEntries(RULES.map((r) => [r.key, 0]));
  for (const r of rules) counts[r]++;
  const noSearch = NO_SEARCH.reduce((s, k) => s + counts[k], 0);
  return { rules, counts, noSearch, total: rules.length };
}

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
  const { rules, counts, total } = settled(games);
  const last = games.lastN;
  const box = host.querySelector('.chart-box');
  const tip = tooltip(box);

  const legend = host.querySelector('.rule-legend');
  legend.innerHTML = legendHTML((key) => counts[key] > 0, counts);

  responsive(box, (width) => {
    const labelW = 46;
    const cols = (width - labelW) / 100 >= 4 ? 100 : 50;
    const pitch = (width - labelW - 2) / cols;
    const gap = pitch >= 7 ? 2 : 1;
    const size = pitch - gap;
    const rows = Math.ceil(total / cols);
    const top = 6;
    const height = top + rows * pitch + 8;
    const svg = freshSVG(box, width, height, `Every game of the chain from 1 to ${last}, a cell each, ${cols} to a row `
      + 'from game 1 at the bottom left, shaded by how it was settled. The counts are in the legend and in the table below.');
    const at = (i) => {
      const r = Math.floor(i / cols);
      return { x: labelW + (i % cols) * pitch, y: top + (rows - 1 - r) * pitch };
    };
    const g = el('g', {}, svg);
    rules.forEach((key, i) => {
      const p = at(i);
      ruleCell(g, key, p.x, p.y, size);
    });
    // The first game of every thousandth row at its left, and the last game after its cell.
    const every = cols === 100 ? 10 : 20;
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
      if (key === 'search') lines.push(['nodes', fmt(c.nodes[i])]);
      tip.show(lines, p.x + size / 2, p.y);
    };
    const hide = () => {
      ring.setAttribute('opacity', 0);
      tip.hide();
    };
    hit.addEventListener('pointermove', (e) => {
      const r = svg.getBoundingClientRect();
      const col = Math.floor((e.clientX - r.left - labelW) / pitch);
      const row = rows - 1 - Math.floor((e.clientY - r.top - top) / pitch);
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
    rows.push([`${fmt(a)} to ${fmt(b)}`, ...each, pct(noSearch / part.length, 0)]);
  }
  rows.push(['all', ...RULES.map((r) => fmt(counts[r.key])), pct(NO_SEARCH.reduce((s, k) => s + counts[k], 0) / total, 0)]);
  tableView(host, 'The numbers behind the chart', ['games', ...RULES.map((r) => r.name), 'no search'], rows);
  return settled(games);
}
