// Along the way: three factors of tax.  Game 40's optimal set, in an order that this figure finds, drawn as a row of
// moves, each over its tax: the moves are the player's (blue circles), their tax the taxman's (orange squares), joined
// by the thick line of a payment, and 30 and the three numbers it takes wear a halo.  A flat figure.
//
// The set and its score come from data/small.json.  The order is found with js/model.js (legalMoves, play, isOver)
// by a depth-first search over the legal orders of the set, which tries the moves in the recorded order and stops at
// the first in which 30 takes three numbers; replay checks that it is a legal game with the recorded score.  On a
// narrow window the row wraps.

import * as M from '../js/model.js';
import { fmt, listWords } from '../js/vocab.js';
import { scene } from '../js/scene.js';
import { responsive } from '../js/chart.js';

const N = 40;
const KEY = 30; // the move that takes three numbers
const WANT = 3;
const COLUMN = 36;
const PAD = 8;
const MOVE_Y = 22;
const TAX_Y = 68;
const TAX_STEP = 32;

/** The first legal order of moves, trying them in the order given, in which the move key takes want numbers as tax;
 *  null if there is none.  The pot after some moves does not depend on their order, so a set of moves left that
 *  failed once fails again, and the search remembers it. */
export function orderWithTax(n, moves, key, want) {
  const failed = new Set();
  let found = null;
  (function search(state, left, path) {
    if (left.length === 0) {
      if (M.isOver(state)) found = path;
      return;
    }
    const id = left.join(',');
    if (failed.has(id)) return;
    const legal = new Set(M.legalMoves(state));
    for (const m of left) {
      if (!legal.has(m)) continue;
      const next = M.play(state, m);
      if (m === key && next.log[next.log.length - 1].tax.length !== want) continue;
      search(next, left.filter((x) => x !== m), [...path, m]);
      if (found) return;
    }
    failed.add(id);
  })(M.newGame(n), moves, []);
  return found;
}

/** Mount the figure in host, an empty element; returns the game's facts. */
export function mountGame40(host, { small }) {
  const game = small.find((g) => g.n === N);
  const order = orderWithTax(N, game.moves, KEY, WANT);
  if (!order) throw new Error(`game ${N}: no order of the optimal set has ${KEY} taking ${WANT} numbers`);
  const played = M.replay(N, order);
  const score = M.score(played);
  if (score !== game.score) throw new Error(`game ${N}: the order found scores ${score}, recorded ${game.score}`);
  const turns = played.log.filter((e) => e.move !== undefined);
  const keyTax = turns.find((t) => t.move === KEY).tax;
  const label = `Game ${N}, played in the order ${order.join(', ')}, each move over its tax. ${KEY} takes ${listWords(keyTax)}. The game scores ${fmt(score)}.`;
  // A row is as tall as its longest tax.
  const heightOf = (row) => TAX_Y + TAX_STEP * (Math.max(...row.map((t) => t.tax.length)) - 1) + 26;

  responsive(host, (width) => {
    // As many columns a row as fit at four fifths of full size, the rows as even as they can be.
    const fit = Math.max(1, Math.floor((width - 2 * PAD) / (COLUMN * 0.8)));
    const cols = Math.ceil(turns.length / Math.ceil(turns.length / fit));
    const rows = [];
    for (let i = 0; i < turns.length; i += cols) rows.push(turns.slice(i, i + cols));
    const tops = rows.map((_, r) => rows.slice(0, r).reduce((sum, row) => sum + heightOf(row), 0));
    host.replaceChildren();
    const height = tops[rows.length - 1] + heightOf(rows[rows.length - 1]);
    const s = scene(host, { width: cols * COLUMN + 2 * PAD, height, label, className: 'is-dense' });
    const look = {};
    turns.forEach((t, i) => {
      const x = PAD + COLUMN * ((i % cols) + 0.5);
      const top = tops[Math.floor(i / cols)];
      const on = t.move === KEY ? { halo: 1 } : {};
      s.number(t.move, t.move);
      look[t.move] = { shape: 1, player: 1, x, y: top + MOVE_Y, ...on };
      t.tax.forEach((d, j) => {
        s.number(d, d);
        look[d] = { shape: 0, taxman: 1, x, y: top + TAX_Y + TAX_STEP * j, ...on };
      });
      s.line(`pay${t.move}`, t.move, t.tax[t.tax.length - 1], 'pay');
      look[`pay${t.move}`] = {};
    });
    s.states([look]).show(0);
  });
  return { order, score, keyTax };
}
