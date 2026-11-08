// Along the way: the cofactor rule.  The games from 188 to the chain's end, grouped by the cofactor of n, n divided by
// its largest prime factor: a bar for each group, split by the rule of each game (js/proof.js), in the cells and tones
// of figures/settled.js.  Only the last group, the cofactors the rule does not name, holds a search.  From data/games.json.
//
// From 188 on, layer 2 of every board is one closed frame (the square theorem, docs/theory/frame-model.md), which is
// the rule's hypothesis (docs/theory/arrival.md, "The cofactor rule").

import { omega, primeFactors } from '../js/model.js';
import { el, fmt } from '../js/vocab.js';
import { freshSVG, pct, responsive, tableView, tooltip } from '../js/chart.js';
import { NO_SEARCH, RULE, RULES, isPrime, ruleOf } from '../js/proof.js';
import { legendHTML, ruleCell } from './settled.js';

export const FROM = 188;

/** The cofactor of n: n divided by its largest prime factor.  3002 = 2 · 19 · 79 has the cofactor 38. */
export function cofactor(n) {
  const ps = primeFactors(n);
  return ps.length ? n / ps[ps.length - 1] : 1;
}

/** The groups of cofactors, the four the rule names and the rest. */
export const KINDS = [
  { key: 'one', name: '1', line: 'n is prime', test: (k) => k === 1 },
  { key: 'prime', name: 'a prime', line: 'as 203 = 7 · 29', test: (k) => isPrime(k) },
  { key: 'twice', name: 'twice a prime', line: 'as 3002 = 38 · 79', test: (k) => k % 2 === 0 && isPrime(k / 2) },
  { key: 'nine', name: '9', line: 'as 1341 = 9 · 149', test: (k) => k === 9 },
  { key: 'other', name: 'any other', line: 'as 200 = 40 · 5', test: () => true },
];

/** The games from FROM on, counted by group and rule, and what the page says of them. */
export function cofactorCounts(games) {
  const c = games.columns;
  const table = Object.fromEntries(KINDS.map((g) => [g.key, Object.fromEntries(RULES.map((r) => [r.key, 0]))]));
  for (let i = FROM - 1; i < c.n.length; i++) {
    const kind = KINDS.find((g) => g.test(cofactor(c.n[i])));
    table[kind.key][ruleOf(c, i)]++;
  }
  const sum = (keys, rules) => keys.reduce((s, g) => s + rules.reduce((t, r) => t + table[g][r], 0), 0);
  const named = KINDS.slice(0, 4).map((g) => g.key);
  const all = KINDS.map((g) => g.key);
  const ruleKeys = RULES.map((r) => r.key);
  return {
    table,
    named: sum(named, ruleKeys),
    namedSearched: sum(named, ['search']),
    namedNoSearch: sum(named, NO_SEARCH),
    noSearch: sum(all, NO_SEARCH),
    total: sum(all, ruleKeys),
    other: sum(['other'], ruleKeys),
    otherSearched: sum(['other'], ['search']),
    from: FROM,
    to: games.lastN,
  };
}

/** The stripes of the chain chart, counted by the chart's own rules (ruleOf), and the claims the page makes of them,
 *  checked here: every game with n % 100 === 0 or n % 100 === 20 was searched; no game from FROM on whose cofactor has
 *  fewer than two prime factors was searched, and the share searched rises from two to three to four or more; every
 *  multiple of 20 that was not searched reached its ceiling; the cofactor of a multiple of 20 above 20 has at least
 *  three prime factors, and that of a multiple of 100 above 100 at least four.  Counts are of prime factors with
 *  multiplicity.  col100 and col20 hold the games with n % 100 === 0 and n % 100 === 20; twenty, those with
 *  n % 20 === 0, with how many reached their ceiling.  byCount groups the games from FROM on by omega(cofactor(n)),
 *  4 standing for four or more.  Each group holds its games and how many were searched. */
export function stripeFacts(games) {
  const c = games.columns;
  const group = () => ({ games: 0, searched: 0 });
  const col100 = group();
  const col20 = group();
  const twenty = { ...group(), ceiling: 0 };
  const byCount = [0, 1, 2, 3, 4].map(group);
  let unexplained = 0;
  let twentyMin = Infinity;
  let hundredMin = Infinity;
  for (let i = 0; i < c.n.length; i++) {
    const n = c.n[i];
    const rule = ruleOf(c, i);
    const searched = rule === 'search' ? 1 : 0;
    const add = (g) => { g.games++; g.searched += searched; };
    const count = omega(cofactor(n));
    if (n % 100 === 0) add(col100);
    if (n % 100 === 20) add(col20);
    if (n % 20 === 0) {
      add(twenty);
      if (rule === 'ceiling') twenty.ceiling++;
      else if (!searched) unexplained++;
      if (n > 20) twentyMin = Math.min(twentyMin, count);
    }
    if (n >= FROM) add(byCount[Math.min(count, 4)]);
    if (n % 100 === 0 && n > 100) hundredMin = Math.min(hundredMin, count);
  }
  const share = (k) => byCount[k].searched / byCount[k].games;
  const claims = [
    [col100.searched === col100.games, 'a game with n % 100 = 0 was not searched'],
    [col20.searched === col20.games, 'a game with n % 100 = 20 was not searched'],
    [byCount[0].searched === 0 && byCount[1].searched === 0,
      'a game whose cofactor has fewer than two prime factors was searched'],
    [share(2) < share(3) && share(3) < share(4), 'the share searched does not rise with the cofactor\'s prime factors'],
    [unexplained === 0, 'a multiple of 20 was neither searched nor at its ceiling'],
    [twentyMin >= 3, 'a multiple of 20 above 20 has a cofactor of fewer than three prime factors'],
    [hundredMin >= 4, 'a multiple of 100 above 100 has a cofactor of fewer than four prime factors'],
  ];
  for (const [holds, claim] of claims) if (!holds) throw new Error(`the stripes: ${claim}`);
  return { col100, col20, twenty, byCount };
}

export function mountCofactor(host, { games }) {
  const facts = cofactorCounts(games);
  const { table } = facts;
  const totals = KINDS.map((g) => RULES.reduce((s, r) => s + table[g.key][r.key], 0));
  const most = Math.max(...totals);
  const box = host.querySelector('.chart-box');
  const tip = tooltip(box);
  host.querySelector('.rule-legend').innerHTML = legendHTML((key) => KINDS.some((g) => table[g.key][key] > 0));

  responsive(box, (width) => {
    const narrow = width < 460;
    const labelW = narrow ? 0 : 150;
    const rowH = narrow ? 50 : 36;
    const barH = 20;
    const endW = 46;
    const headY = 14;
    const top = 22;
    const height = top + KINDS.length * rowH + 4;
    const svg = freshSVG(box, width, height, `The games from game ${fmt(FROM)} to game ${fmt(games.lastN)} by the cofactor of n, each bar `
      + 'splitting its games by how we settled them. Only the games of other cofactors include searches. The numbers are in '
      + 'the table below.');
    const x0 = labelW;
    const scale = (width - labelW - endW) / most;
    const axes = el('g', { class: 'axis' }, svg);
    el('text', { x: narrow ? 0 : labelW - 12, y: headY, 'text-anchor': narrow ? 'start' : 'end' }, axes)
      .textContent = 'cofactor';
    KINDS.forEach((g, r) => {
      const y = top + r * rowH + (narrow ? 20 : (rowH - barH) / 2);
      const name = el('text', {
        x: narrow ? 0 : labelW - 12, y: narrow ? y - 7 : y + barH / 2 - 2, 'text-anchor': narrow ? 'start' : 'end',
        class: 'row-label',
      }, svg);
      name.textContent = g.name;
      if (!narrow) {
        const line = el('text', { x: labelW - 12, y: y + barH / 2 + 12, 'text-anchor': 'end', class: 'row-count' }, svg);
        line.textContent = g.line;
      } else name.textContent = `${g.name}: ${g.line}`;
      let x = x0;
      for (const rule of RULES) {
        const k = table[g.key][rule.key];
        if (!k) continue;
        const w = k * scale;
        const at = x;
        const seg = ruleCell(svg, rule.key, at, y, Math.max(1, w - 2), barH);
        seg.classList.add('is-segment');
        const rows = [`${fmt(k)} games`, ['cofactor', g.name], ['settled by', RULE[rule.key].name]];
        const show = () => tip.show(rows, at + w / 2, y);
        seg.addEventListener('pointerenter', show);
        seg.addEventListener('pointerleave', () => tip.hide());
        if (rule.key === 'search' && w > 70) {
          const t = el('text', { x: x + 8, y: y + barH / 2 + 4, class: 'rule-in' }, svg);
          t.textContent = `${fmt(k)} searched`;
        }
        x += w;
      }
      const total = el('text', { x: x + 4, y: y + barH / 2 + 4, class: 'row-count' }, svg);
      total.textContent = fmt(totals[r]);
    });
  });

  const head = ['cofactor', ...RULES.map((r) => r.name), 'games'];
  const rows = KINDS.map((g, r) => [g.name, ...RULES.map((x) => fmt(table[g.key][x.key])), fmt(totals[r])]);
  tableView(host, 'The numbers behind the chart', head, rows);
  return { ...facts, share: facts.namedNoSearch / facts.noSearch, pctShare: pct(facts.namedNoSearch / facts.noSearch, 0) };
}
