// The problem shrinking, step by step, for the last game of the chain: one row a step, with its title (a link once the
// chapter is written), the point it makes, and a bar for what is left to choose after it, on a log scale, labeled with
// its count.  The rows of chapters not yet written are quieter.  Chapter 2's row gives, in words, how near the best the
// game before starts the search (a measurement, from js/data.js).  Chapter 6's bound is the search's, so chapter 5's
// row counts what it leaves; chapters 7 and 8 are about the whole chain, not one game's problem.  None of the three
// has a row.
//
// The chapters come from js/site.js; every count is computed from data/games.json.  The counts are rough sizes, as the
// page's note says: n! orders, 2^n sets, 2^(n/2) sets of the numbers up to n/2, 2^c sets of the c candidates, and the
// nodes of the recorded search.  A bar's length is the count's number of digits, so a count of 10^1,523 has a bar a
// tenth as long as one of 10^15,230.

import { STORY, hrefOf } from '../js/site.js';
import { fmt } from '../js/vocab.js';
import { headStart } from '../js/data.js';

/** The point of each chapter, as the row says it: what it takes away. */
const POINTS = {
  game: 'A game is an order of moves.',
  1: 'Only the set matters, and a set is quick to check.',
  3: 'One greedy pass chooses the numbers above n/2.',
  4: 'Only the promotions are left to choose.',
  5: 'A search with a sharp bound finishes.',
  2: 'Using the game before starts the search near opt(n).',
};

/** The first row is the game as played, not a chapter: no number, and a link to the front page's section on it. */
const GAME_ROW = { title: 'The game', href: '#the-game' };

/** The chapters with a row, in the order of the rows. */
const STEPS = [1, 3, 4, 2, 5];

/** The links of chapters whose row goes somewhere other than the chapter's page: chapter 2's row, the start that game
 *  n − 1 gives a search, goes to the section of chapter 5 that explains it. */
const HREF = { 2: 'search.html#head-start' };

const LOG2 = Math.log10(2);

/** log10 n!, summed: the error is far below what the floor of a count of thousands of digits could notice. */
export function log10Factorial(n) {
  let sum = 0;
  for (let k = 2; k <= n; k++) sum += Math.log10(k);
  return sum;
}

// A count of about 10^exponent, where exponent is log10 of the count rounded to the nearest power, with the noun it
// counts: a row's bar.
const power = (log, noun) => ({ log, exponent: Math.round(log), html: `10<sup>${fmt(Math.round(log))}</sup> ${noun}` });

// A large count in words: 3,179,486,263 as 3.2 billion.
function inWords(x) {
  for (const [size, word] of [[1e12, 'trillion'], [1e9, 'billion'], [1e6, 'million']]) {
    if (x >= size) return `${Number((x / size).toPrecision(2))} ${word}`;
  }
  return fmt(x);
}

/** What is left to choose in game n as played (key game) and after each chapter, keyed by chapter: a bar
 *  { log, exponent, html }, words alone { html }, or nothing. */
export function shrinkCounts(games) {
  const c = games.columns;
  const n = games.lastN;
  const i = c.n.indexOf(n);
  const start = headStart(games);
  const candidates = c.candidates[i];
  const nodes = c.nodes[i];
  const log1 = log10Factorial(n);
  const log2 = n * LOG2;
  const log4 = Math.floor(n / 2) * LOG2;
  return {
    n,
    game: power(log1, 'orders'),
    1: power(log2, 'sets'),
    3: power(log4, 'sets'),
    4: candidates == null
      ? { html: 'nothing to promote' }
      : power(candidates * LOG2, `sets of ${fmt(candidates)} candidates`),
    5: nodes == null ? { html: 'no search at all' } : { log: Math.log10(nodes), html: `${inWords(nodes)} nodes` },
    2: { html: `the search starts only ${fmt(start.seededGap)} points below opt(n)` },
  };
}

export function mountShrink(host, { games }) {
  const counts = shrinkCounts(games);
  const max = counts.game.log;
  const chapters = STEPS.map((n) => STORY.find((chapter) => chapter.n === n));
  const rows = [GAME_ROW, ...chapters].map((chapter) => {
    const key = chapter.n ?? 'game';
    const href = HREF[chapter.n] ?? chapter.href ?? hrefOf(chapter);
    const count = counts[key];
    const title = href
      ? `<a class="shrink-title" href="${href}">${chapter.title}</a>`
      : `<span class="shrink-title">${chapter.title}<span class="sr-only">, not yet written</span></span>`;
    let measure = '';
    if (count?.log !== undefined) {
      const f = count.log / max;
      const label = `<span class="shrink-count">${count.html}</span>`;
      // A bar long enough to hold its count holds it; any other has it at its tip.
      measure = f > 0.5
        ? `<span class="shrink-bar is-labeled" style="--f: ${f.toFixed(5)}">${label}</span>`
        : `<span class="shrink-bar" style="--f: ${f.toFixed(5)}"></span>${label}`;
    } else if (count) measure = `<span class="shrink-count">${count.html}</span>`;
    return `<li class="shrink-row ${href ? 'is-written' : 'is-unwritten'}">
      <span class="shrink-text">${title}<span class="shrink-point">${POINTS[key]}</span></span>
      <span class="shrink-measure">${measure}</span>
    </li>`;
  }).join('');
  host.innerHTML = `<p class="shrink-head"><span class="shrink-head-title">What is left to choose in game
    ${fmt(counts.n)}</span>
    <span class="shrink-head-sub">on a log scale: a bar is as long as its count has digits</span></p>
    <ol class="shrink-rows">${rows}</ol>`;
  return counts;
}
