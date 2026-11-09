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
import { COLS, legendHTML, ruleCell } from './settled.js';

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

/** The stripes of the chain chart (COLS games to a row), counted by the chart's own rules (ruleOf), and the claims the
 *  page makes of them, checked here.  A column is named by its first game r, 1 to COLS, and holds the games with
 *  (n - 1) % COLS + 1 === r.  darkColumns are the r whose games were all searched, lightColumns the r whose games
 *  include no search.  dark counts the games whose n is divisible by 8, 12, 20 or 30, with how many were searched and
 *  how many reached their ceiling; light, those whose n is twice a number with no factor 2, 3 or 5, with how many were
 *  searched and how many have a cofactor that is a prime or twice a prime (named).  byLayer groups the games from FROM
 *  on by their layer, omega(n), the count of prime factors with multiplicity, 5 standing for five or more; each group
 *  holds its layer, its games and how many were searched.
 *
 *  Claims: both lists of columns are non-empty; every dark column's r is divisible by 8, 12, 20 or 30; every light
 *  column's r is twice a number with no factor 2, 3 or 5, and all its games reached their ceiling; every dark game was
 *  searched or at its ceiling; every dark n other than 8, 12, 20 and 30 lies in layer 4 or above; no game from FROM on
 *  in layer 1 or 2 was searched, and the share searched rises from layer 3 to layer 4 to layer 5 or more. */
export function stripeFacts(games) {
  const c = games.columns;
  const group = () => ({ games: 0, searched: 0 });
  const isDark = (n) => n % 8 === 0 || n % 12 === 0 || n % 20 === 0 || n % 30 === 0;
  const isLight = (n) => n % 2 === 0 && n % 4 !== 0 && n % 3 !== 0 && n % 5 !== 0;
  const columns = Array.from({ length: COLS + 1 }, () => ({ games: 0, searched: 0, ceiling: 0 }));
  const dark = { ...group(), ceiling: 0 };
  const light = { ...group(), named: 0 };
  const byLayer = [1, 2, 3, 4, 5].map((layer) => ({ layer, ...group() }));
  const namedKinds = KINDS.slice(0, 4);
  const claims = [];
  const claim = (holds, text) => { if (!holds) claims.push(text); };
  for (let i = 0; i < c.n.length; i++) {
    const n = c.n[i];
    const rule = ruleOf(c, i);
    const searched = rule === 'search' ? 1 : 0;
    const add = (g) => { g.games++; g.searched += searched; };
    const col = columns[((n - 1) % COLS) + 1];
    add(col);
    if (rule === 'ceiling') col.ceiling++;
    const k = cofactor(n);
    if (isDark(n)) {
      add(dark);
      if (rule === 'ceiling') dark.ceiling++;
      claim(searched || rule === 'ceiling',
        `game ${n}, divisible by 8, 12, 20 or 30, was neither searched nor at its ceiling`);
      if (![8, 12, 20, 30].includes(n)) {
        claim(omega(n) >= 4, `game ${n}, divisible by 8, 12, 20 or 30, lies below layer 4`);
      }
    }
    if (isLight(n)) {
      add(light);
      if (namedKinds.slice(1, 3).some((g) => g.test(k))) light.named++;
    }
    if (n >= FROM) add(byLayer[Math.min(omega(n), 5) - 1]);
  }
  const darkColumns = [];
  const lightColumns = [];
  for (let r = 1; r <= COLS; r++) {
    const col = columns[r];
    if (col.games && col.searched === col.games) darkColumns.push(r);
    if (col.games && col.searched === 0) lightColumns.push(r);
  }
  claim(darkColumns.length > 0, 'no column was searched in every game');
  claim(lightColumns.length > 0, 'no column was searched in no game');
  for (const r of darkColumns) {
    claim([8, 12, 20, 30].some((d) => r % d === 0), `dark column ${r} is not divisible by 8, 12, 20 or 30`);
  }
  for (const r of lightColumns) {
    claim(isLight(r), `light column ${r} is not twice a number with no factor 2, 3 or 5`);
    claim(columns[r].ceiling === columns[r].games, `a game in light column ${r} is not at its ceiling`);
  }
  const share = (g) => g.searched / g.games;
  claim(byLayer[0].searched === 0 && byLayer[1].searched === 0, 'a game in layer 1 or 2 was searched');
  claim(share(byLayer[2]) < share(byLayer[3]) && share(byLayer[3]) < share(byLayer[4]),
    'the share searched does not rise with the layer');
  if (claims.length) throw new Error(`the stripes: ${claims[0]}`);
  return { byLayer, darkColumns, lightColumns, dark, light };
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
