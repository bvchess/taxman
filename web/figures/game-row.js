// A game of n drawn as a row of moves, each over its tax, and at the right of the row what is left in the pot.  The
// moves are the player's (blue circles), their tax the taxman's (orange squares), joined by the thick line of a payment
// (as in figures/game40.js); a move that takes several numbers stacks them.  The numbers left in the pot at the end are
// out of focus, as in figures/arrival.js: a dashed circle for a number above n/2 and a square for one up to n/2, and a
// move given up is a dashed circle with a halo.  A halo marks what the prose beside the figure names.  Two rows can
// share their columns, an empty column standing where a move of the other row has no move of this one.  A flat figure.
//
// The game is replayed with js/model.js (replay), so a row cannot show an illegal game: the taxes and the numbers left
// are the model's, and so is the score.  The figures that draw rows (figures/slip.js, figures/fits.js) take their moves
// from data/small.json.

import * as M from '../js/model.js';

export const COLUMN = 36;
export const PAD = 12;
export const MOVE_Y = 22;
export const TAX_Y = 68;
export const TAX_STEP = 32;
export const POT_GAP = 0.6; // the gap, in columns, between the moves and the pot

/** A game of n played with the moves, in order: { n, moves, turns: [{ move, tax }], rest, score }, rest being what the
 *  taxman takes at the end.  Throws if a move is illegal. */
export function gameRow(n, moves) {
  const played = M.replay(n, moves);
  const turns = played.log.filter((e) => e.move !== undefined);
  return { n, moves, turns, rest: played.log[played.log.length - 1].rest, score: M.score(played) };
}

/** The x of the middle of column c, columns starting at left. */
export const columnX = (c, left = PAD) => left + COLUMN * (c + 0.5);

/** The width of a row of the given columns, the pot of the given size and the row's label of the given width. */
export const rowWidth = (columns, pot, label = 0) => 2 * PAD + label + COLUMN * (columns + POT_GAP + pot);

/** The row of the slip figure, which every figure of rows is drawn beside: six columns and a pot of two, drawn at most
 *  this many rem wide.  A figure of rows is drawn at the same size of mark, so that the marks look alike from figure to
 *  figure. */
export const REFERENCE = { columns: 6, pot: 2, rem: 26 };

/** The screen pixels of one figure unit of a row of the reference's size, in a space of the given width. */
export function unitPx(width) {
  const rem = parseFloat(getComputedStyle(document.documentElement).fontSize);
  return Math.min(width, REFERENCE.rem * rem) / rowWidth(REFERENCE.columns, REFERENCE.pot);
}

/** The height of rows of the given taxes: as tall as the longest of them. */
export const rowHeight = (rows) => TAX_Y + TAX_STEP * (Math.max(...rows.flatMap((r) => r.turns.map((t) => t.tax.length))) - 1) + 20;

/** The id of the line that pays for move m in the row tagged tag; numbers are their own ids when there is no tag. */
export const idOf = (tag, x) => (tag ? `${tag}${x}` : x);
export const payId = (tag, m) => `${tag}pay${m}`;

/** The look of a number left in the pot of game n, or given up: a dashed circle above n/2, a square up to it. */
export function potLook(x, n, { given = false } = {}) {
  if (given) return { shape: 1, dashed: 1, halo: 1 };
  return x > n / 2 ? { shape: 1, dashed: 1, faded: 1 } : { shape: 0, faded: 1 };
}

/** The numbers to register in a scene for a row: its moves, their taxes and what is left. */
export const numbersOf = (row) => [...row.turns.flatMap((t) => [t.move, ...t.tax]), ...row.rest];

/** Register the marks of row in scene s: its numbers and a payment line from each move to its last tax. */
export function addRow(s, row, tag = '') {
  for (const x of numbersOf(row)) s.number(idOf(tag, x), x);
  for (const t of row.turns) s.line(payId(tag, t.move), idOf(tag, t.move), idOf(tag, t.tax[t.tax.length - 1]), 'pay');
}

/** The looks of a row's marks, an object to put into a state.
 *    slots   the move at each column (null for an empty column): a row's moves stand in the columns of their slots
 *    left    the x where column 0 begins;  top, the y of the row's moves;  tag, the prefix of its ids
 *    halo    the numbers that wear a halo
 *    pot     the column at which the pot's first number stands (default: after the last slot) */
export function rowLooks(row, { slots, left = PAD, top = 0, tag = '', halo = new Set(), pot = slots.length + POT_GAP }) {
  const looks = {};
  const wear = (x) => (halo.has(x) ? { halo: 1 } : {});
  for (const t of row.turns) {
    const c = slots.indexOf(t.move);
    if (c < 0) throw new Error(`game ${row.n}: the move ${t.move} has no column`);
    const x = columnX(c, left);
    looks[idOf(tag, t.move)] = { shape: 1, player: 1, x, y: top + MOVE_Y, ...wear(t.move) };
    t.tax.forEach((d, j) => {
      looks[idOf(tag, d)] = { shape: 0, taxman: 1, x, y: top + TAX_Y + TAX_STEP * j, ...wear(d) };
    });
    looks[payId(tag, t.move)] = {};
  }
  row.rest.forEach((x, k) => {
    looks[idOf(tag, x)] = { ...potLook(x, row.n), x: columnX(pot + k, left), y: top + MOVE_Y, ...wear(x) };
  });
  return looks;
}
