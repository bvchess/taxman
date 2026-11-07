// Does n fit?  Two panels of game rows, one for a game n that fits and one for a game n that does not: each shows the
// optimal game of game n − 1, above, and a game of n, below, in the same columns, each move over its tax and at the
// right what is left out (figures/game-row.js).  When n fits, the game below is the one above with n played
// last, paid for by a number the game above left in the pot.  When it does not, it is game n's optimal game, which
// gives up a move of the game above so that the move can pay for n.  A halo marks what n uses and what changes.  A flat
// figure.
//
// The games are data/small.json's, and the panels' words are worked out from them: whether n fits is decided by
// peel (js/model.js, isPlayable), the maximal factors of n by js/model.js, the taxes and the scores by replaying the
// moves.  The figure checks that game n scores its ceiling, opt(n − 1) + n, exactly when n fits.
//
// The panel of a prime n (mountPrime) is drawn by the same code: game n − 1's optimal game above, and below it the
// same game with its first move, the largest prime below n, replaced by n.

import * as M from '../js/model.js';
import { scene } from '../js/scene.js';
import { fmt, listWords } from '../js/vocab.js';
import { slipFacts } from './slip.js';
import { responsive } from '../js/chart.js';
import { HEADER_H, addRow, gameRow, rowHeight, rowLooks, rowWidth, unitPx } from './game-row.js';

const ascending = (a, b) => a - b;
const sum = (list) => list.reduce((total, x) => total + x, 0);

/** The facts of the panel of n, from the recorded games: { n, fits, above, below, uses, gaveUp, slots }.  above is
 *  game n − 1's optimal game, below a game of n (the one above with n played in when n fits, game n's own when not),
 *  uses the numbers of the game above that pay for n, gaveUp the moves the game below gives up, and slots the move of
 *  each column.  Throws if game n's score is not opt(n − 1) + n exactly when n fits. */
export function fitsFacts(small, n) {
  const prev = small[n - 2];
  const cur = small[n - 1];
  const fail = (why) => {
    throw new Error(`fits of ${n}: ${why}`);
  };
  if (prev?.n !== n - 1 || cur?.n !== n) fail('small.json has no games n − 1 and n');
  const fits = M.isPlayable([...prev.moves, n]);
  if (fits !== (cur.score === prev.score + n)) {
    fail(`n ${fits ? 'fits' : 'does not fit'}, but game ${n} scores ${cur.score}`);
  }
  const above = gameRow(n - 1, prev.moves);
  if (above.score !== prev.score) fail(`game ${n - 1}'s record scores ${prev.score}`);
  let below;
  if (fits) {
    const slip = slipFacts(n, prev.moves);
    if (slip.dropped !== null || slip.after.moves.join() !== [...prev.moves, n].join()) fail('n does not go last');
    below = slip.after;
  } else below = gameRow(n, cur.moves);
  if (below.score !== cur.score) fail(`the game below scores ${below.score}, not ${cur.score}`);
  const tax = below.turns.find((t) => t.move === n).tax;
  const gaveUp = above.moves.filter((m) => !below.moves.includes(m));
  const uses = fits ? tax : M.maximalFactors(n).sort(ascending);
  if (!fits && (below.score !== prev.score + n - sum(gaveUp))) fail('the game below does not give up what it loses');
  // The columns of the game above, then the moves only the game below has.
  const slots = [...above.moves, ...below.moves.filter((m) => !above.moves.includes(m))];
  return { n, fits, prev, cur, above, below, tax, uses, gaveUp, slots };
}

// How each number that can pay for n stands in the game above: a move, tax of a move, or left in the pot.
function standing(above, x) {
  const payer = above.turns.find((t) => t.tax.includes(x));
  if (payer) return `${x} pays for ${payer.move}`;
  return above.moves.includes(x) ? `${x} is a move` : `the game leaves ${x} out`;
}

/** The panel's words as HTML: the lead in bold, then why. */
export function fitsWords(f) {
  const { n, above, tax, uses, gaveUp, fits, prev, cur } = f;
  if (fits) {
    return `<strong>${n} fits.</strong> Game ${n - 1}'s optimal game leaves ${listWords(tax)} out, and ${n} `
      + `follows it, with ${listWords(tax)} paying for it: ${fmt(prev.score)} + ${n} = ${fmt(cur.score)}, the ceiling.`;
  }
  const how = uses.map((x) => standing(above, x)).join(' and ');
  const score = `${fmt(prev.score)} + ${n} − ${gaveUp.join(' − ')} = ${fmt(cur.score)}`;
  return `<strong>${n} does not fit.</strong> Only ${listWords(uses)}, its maximal factors, can pay for it, and in `
    + `game ${n - 1}'s optimal game ${how}.  Game ${n}'s optimal game gives up ${listWords(gaveUp)}, which pays for `
    + `${n}: ${score}.`;
}

/** The facts of the panel of a prime n, from the recorded games: { n, q, prev, cur, above, below, tax, slots }.  above
 *  is game n − 1's optimal game, whose first move is q, the largest prime below n, and below is that game with q
 *  replaced by n, which scores opt(n − 1) + n − q = opt(n); tax is what n takes and slots the move of each column of
 *  each row (n stands in q's column).  Throws if any of this fails. */
export function primeFacts(small, n) {
  const prev = small[n - 2];
  const cur = small[n - 1];
  const fail = (why) => {
    throw new Error(`prime panel of ${n}: ${why}`);
  };
  if (prev?.n !== n - 1 || cur?.n !== n) fail('small.json has no games n − 1 and n');
  if (n < 2 || M.omega(n) !== 1) fail('n is not a prime');
  let q = n - 1;
  while (q > 1 && M.omega(q) !== 1) q--;
  const above = gameRow(n - 1, prev.moves);
  if (above.score !== prev.score) fail(`game ${n - 1}'s record scores ${prev.score}`);
  if (above.moves[0] !== q) fail(`the first move of game ${n - 1}'s optimal game is ${above.moves[0]}, not ${q}`);
  const below = gameRow(n, above.moves.map((m) => (m === q ? n : m)));
  if (below.score !== cur.score) fail(`the game below scores ${below.score}, not ${cur.score}`);
  if (cur.score !== prev.score + n - q) fail(`game ${n} scores ${cur.score}, not opt(n − 1) + n − ${q}`);
  const tax = below.turns.find((t) => t.move === n).tax;
  return { n, q, prev, cur, above, below, tax, slots: [above.moves, below.moves] };
}

/** The prime panel's words as HTML: the lead in bold, then why. */
export function primeWords(f) {
  const { n, q, prev, cur, above } = f;
  return `<strong>${n} is a prime.</strong>  Its only maximal factor is ${listWords(M.maximalFactors(n))}, so it can `
    + `only be the first move.  It takes the place of ${q}, the first move of game ${above.n}'s optimal game, and `
    + `every other move stays the same: ${fmt(prev.score)} + ${n} − ${q} = ${fmt(cur.score)}.`;
}

const rowName = (row) => `Game ${row.n}`;
const panelLabel = (f) => `${rowName(f.above)} and game ${f.n}, each move over the tax it takes.`;
const plain = (html) => html.replace(/<[^>]+>/g, '');

const ROWS_GAP = 14; // between a row's tax and the next row's header
const SIDE = 0.85; // the panels stand side by side when each keeps this much of the slip figure's size
const SPACE = 22; // between the panels, in px

/** The rows of a panel of fits: the optimal game of game n − 1, above, and a game of n below, in the same columns. */
const fitsRows = (f) => [
  { tag: 'a', row: f.above, halo: new Set(f.uses), slots: f.slots },
  { tag: 'b', row: f.below, halo: new Set([f.n, ...f.tax]), slots: f.slots },
];

/** The rows of the panel of a prime: game n − 1's optimal game, above, with q and its tax, and below it the game with n
 *  in q's place, with n and its tax; q, given up, is left out beside the numbers the game below leaves. */
const primeRows = (f) => [
  { tag: 'a', row: f.above, halo: new Set([f.q, ...f.above.turns[0].tax]), slots: f.slots[0] },
  { tag: 'b', row: f.below, halo: new Set([f.n, ...f.tax]), slots: f.slots[1], given: new Set([f.q]) },
];

/** Draw the panels of fig: each .arrival in it with an svg and an .arrival-says, a panel being { host, words, rows,
 *  label }, its rows { tag, row, halo, slots, given } drawn one above the other, each under its header line: the row's
 *  name, and "left out" over what it leaves.  The panels are drawn at the slip figure's size, side by side if each
 *  keeps SIDE of it and stacked if not, and again when the figure's width changes.  Returns a highlight for the
 *  prose. */
function mountRows(fig, panels) {
  const columns = Math.max(...panels.flatMap((p) => p.rows.map((r) => r.slots.length)));
  const pot = Math.max(...panels.flatMap((p) => p.rows.map((r) => r.row.rest.length)));
  // Every panel is drawn at one scale: the widest sets the width.
  const width = rowWidth(columns, pot);
  const scenes = panels.map(({ host, words, rows, label }) => {
    // Each row is as tall as its own longest tax, so the gap under every row is the same.
    const heights = rows.map(({ row }) => HEADER_H + rowHeight([row]) + ROWS_GAP);
    const s = scene(host.querySelector('svg'), {
      width, height: heights.reduce((a, b) => a + b) - ROWS_GAP, className: 'is-dense',
      label: `${label} ${plain(words)}`,
    });
    const looks = {};
    let top = 0;
    rows.forEach(({ tag, row, halo, slots, given }, i) => {
      const header = { name: rowName(row), out: row.rest.length > 0 };
      addRow(s, row, tag, header);
      Object.assign(looks, rowLooks(row, { slots, top: top + HEADER_H, tag, halo, given, header }));
      top += heights[i];
    });
    s.states([looks]).show(0);
    const says = host.querySelector('.arrival-says');
    if (says) says.innerHTML = words;
    return s;
  });
  responsive(fig, (space) => {
    const unit = unitPx(space);
    const each = (space - SPACE) / 2;
    fig.classList.toggle('is-side', panels.length > 1 && each >= SIDE * unit * width);
    const avail = fig.classList.contains('is-side') ? each : space;
    for (const { host } of panels) host.querySelector('svg').style.width = `${Math.min(width * unit, avail)}px`;
  });
  return {
    highlight: (x, on) => scenes.forEach((s) => ['a', 'b'].forEach((tag) => s.highlight(`${tag}${x}`, on))),
  };
}

/** Mount the panels of fig: each .arrival in it with a data-board n.  Returns the facts of each panel and a highlight
 *  for the prose. */
export function mountFits(fig, { small }) {
  const panels = [...fig.querySelectorAll('.arrival')].map((host) => {
    const f = fitsFacts(small, Number(host.dataset.board));
    return { host, f, words: fitsWords(f), rows: fitsRows(f), label: panelLabel(f) };
  });
  return { facts: panels.map(({ f }) => f), ...mountRows(fig, panels) };
}

/** Mount the panel of fig: its one .arrival, with a data-board n that is a prime.  Returns the panel's facts and a
 *  highlight for the prose. */
export function mountPrime(fig, { small }) {
  const host = fig.querySelector('.arrival');
  const f = primeFacts(small, Number(host.dataset.board));
  const panel = { host, f, words: primeWords(f), rows: primeRows(f), label: panelLabel(f) };
  return { facts: f, ...mountRows(fig, [panel]) };
}
