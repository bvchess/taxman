// The arrival of n: the board of game n, ten numbers to a row with 1 at the bottom left, so that the board grows upward
// as n does.  The moves of an optimal game of n are the player's (blue circles); the other numbers are out of focus, a
// dashed circle for a number above n/2 that is not played and a square for one up to n/2.  A halo marks what changed
// from game n − 1: n itself and any move that joins it, and, dashed, a move that game n gives up.  A flat figure.
//
// The sets and scores come from data/small.json and the record from data/games.json.  Whether n fits, that is,
// whether the optimal set of game n − 1 with n added can be played, is decided by peel (js/model.js, isPlayable), and
// the figure checks what it says against the recorded games: a game n fits exactly when it scores its ceiling, and a
// prime takes the place of the largest prime below it.

import * as M from '../js/model.js';
import { scene } from '../js/scene.js';
import { fmt, listWords } from '../js/vocab.js';
import { isPrime, recordWords, repeatOf, ruleOf } from './settled.js';

export const COLS = 10;
const PITCH = 36;
const ROW = 35;
const PAD = 16;
const SCALE = 1;
const R = 15 * SCALE; // a circle's radius

const ascending = (a, b) => a - b;

/** The size of a board of the given rows, in figure units. */
export const boardSize = (rows) => ({ width: 2 * PAD + COLS * PITCH, height: 2 * PAD + 2 * R + ROW * (rows - 1) });

/** The center of number x on a board of the given rows: 1 at the bottom left, ten to a row. */
export function cell(x, rows) {
  const c = (x - 1) % COLS;
  const r = Math.floor((x - 1) / COLS);
  return { x: PAD + PITCH * (c + 0.5), y: PAD + R + ROW * (rows - 1 - r) };
}

function largestPrimeBelow(n) {
  for (let q = n - 1; q >= 2; q--) if (isPrime(q)) return q;
  return null;
}

/** What happens when n arrives, for 2 ≤ n ≤ the last game of small.json: the sets of n − 1 and n, whether n fits,
 *  the moves that leave and join, the scores and the chain's record.  Throws if the recorded games disagree with
 *  it. */
export function arrival({ small, games }, n) {
  const game = (k) => small[k - 1];
  const fail = (why) => {
    throw new Error(`arrival of ${n}: ${why}`);
  };
  if (game(n)?.n !== n || game(n - 1)?.n !== n - 1) fail(`small.json has no games ${n - 1} and ${n}`);
  const before = new Set(game(n - 1).moves);
  const after = new Set(game(n).moves);
  const prevScore = game(n - 1).score;
  const score = game(n).score;
  const ceiling = prevScore + n;
  const prime = isPrime(n);
  const q = prime ? largestPrimeBelow(n) : null;
  const fits = M.isPlayable([...before, n]);
  const left = [...before].filter((x) => !after.has(x)).sort(ascending);
  const joined = [...after].filter((x) => !before.has(x) && x !== n).sort(ascending);
  const c = games.columns;
  const i = n - 1;
  if (c.n[i] !== n || c.score[i] !== score) fail('games.json and small.json disagree');
  if (fits && score !== ceiling) fail('n fits, but the game is below its ceiling');
  if (!fits && score === ceiling) fail('n does not fit, but the game reaches its ceiling');
  if (fits && (left.length || joined.length)) fail(`n fits, but the recorded set is not game ${n - 1}'s with n`);
  if (q && (left.join() !== String(q) || joined.length || !after.has(n))) {
    fail(`the recorded set is not game ${n - 1}'s with ${q} exchanged for n`);
  }
  return {
    n, before, after, prevScore, score, ceiling, gap: ceiling - score, prime, q, fits, left, joined,
    rule: ruleOf(c, i), proof: c.proof[i], record: recordWords(c, i),
    repeats: c.proof[i] === 'same_search' ? repeatOf(n) : null,
  };
}

/** The looks of the numbers 1 to n of game n's board, of the given rows. */
export function boardLooks(f, rows) {
  const looks = {};
  for (let x = 1; x <= f.n; x++) {
    const at = { ...cell(x, rows), scale: SCALE };
    if (f.after.has(x)) looks[x] = { ...at, shape: 1, player: 1, halo: x === f.n || f.joined.includes(x) ? 1 : 0 };
    else if (f.left.includes(x)) looks[x] = { ...at, shape: 1, dashed: 1, halo: 1 };
    else if (x > f.n / 2) looks[x] = { ...at, shape: 1, dashed: 1, faded: 1 };
    else looks[x] = { ...at, shape: 0, faded: 1 };
  }
  return looks;
}

/** What happened when n arrived, in words, as HTML: the lead in bold, then the step.  With record, the chain's proof
 *  follows; short gives the change and the score alone, for a board beside the prose that explains it. */
export function arrivalWords(f, { record = false, short = false } = {}) {
  const { n } = f;
  const was = n - 1;
  const half = Math.floor(n / 2);
  const takers = [n, ...f.joined];
  const take = `${listWords(takers)} take${takers.length === 1 ? 's' : ''} the place of ${listWords(f.left)}`;
  const below = `${fmt(f.score)}, ${fmt(f.gap)} below the ceiling`;
  const swap = `${fmt(f.prevScore)} + ${n} − ${f.q} = ${fmt(f.score)}`;
  const added = `${fmt(f.prevScore)} + ${n} = ${fmt(f.score)}`;
  if (short) {
    if (f.prime && f.q) return `${n} takes the place of ${f.q}, and the game scores ${swap}.`;
    if (f.fits) return `${n} fits, and the game scores ${added}, the ceiling.`;
    const alone = takers.length === 1 ? `it takes the place of ${listWords(f.left)}` : take;
    return `${n} does not fit: ${alone}, and the game scores ${below}.`;
  }
  let lead;
  let text;
  if (n === 2) {
    lead = '2 is the first prime.';
    text = 'Game 1 has no move, so game 2 opens with 2, paid for by 1, and scores 2.';
  } else if (f.prime) {
    lead = `${n} is prime.`;
    text = `It takes the place of ${f.q}, the largest prime below it, and the rest of game ${was}’s set `
      + `stays.  The game scores ${swap}.`;
  } else if (f.fits) {
    lead = `${n} fits.`;
    text = `Game ${was}’s optimal set, with ${n} added, can still be played.  It scores ${added}, the ceiling, so it is `
      + 'optimal.';
  } else {
    lead = `${n} does not fit.`;
    text = `Game ${was}’s optimal set, with ${n} added, cannot be played.  In game ${n}’s optimal set, ${take}, `
      + `and it scores ${below}.`;
  }
  let proof = '';
  if (record) {
    // A game that fits is proved by the ceiling too, and the record's proof gives the same game.
    const same = ', which gives the same game';
    const carries = `: game ${n} poses the same search problem, so game ${f.repeats}’s answer carries over`;
    const nothing = `: once the numbers above ${half} are chosen, no number up to ${half} can join them`;
    const how = {
      prime_formula: 'by the prime formula',
      cap: 'at its ceiling',
      same_search: `as a repeat of game ${f.repeats}’s search${f.fits ? same : carries}`,
      exhausted: `by ${f.record}`,
      trivial: `with nothing to choose${f.fits ? same : nothing}`,
    }[f.proof] ?? `by ${f.record}`;
    proof = `<span class="walk-record">Proved in the chain ${how}.</span>`;
  }
  return `<strong>${lead}</strong> ${text}${proof ? ` ${proof}` : ''}`;
}

/** Draw game n's board, static, into host's <svg>, and say what changed in says, by default host's .arrival-says. */
export function mountArrival(host, data, n, { says = host.querySelector('.arrival-says') } = {}) {
  const f = arrival(data, n);
  const rows = Math.ceil(n / COLS);
  const { width, height } = boardSize(rows);
  const svg = host.querySelector('svg');
  const plain = arrivalWords(f, { short: true }).replace(/<[^>]+>/g, '');
  const sc = scene(svg, { width, height, label: `The board of game ${n}. ${plain}` });
  for (let x = 1; x <= n; x++) sc.number(x, x);
  sc.states([boardLooks(f, rows)]).show(0);
  if (says) says.innerHTML = `<strong>Game ${n}.</strong> ${arrivalWords(f, { short: true })}`;
  return { facts: f, highlight: (x, on) => sc.highlight(x, on) };
}
