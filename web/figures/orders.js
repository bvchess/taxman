// Sets, not orders: the nine moves of game 21's recorded game, each above the factor that pays for it, in every
// order that is a legal game.  A flat figure: the moves are the player's (blue circles), their tax the taxman's
// (orange squares), joined by the thick line of a payment.  The orders are found with js/model.js (legalMoves,
// play, isOver) by a search over the moves of the set, and each order's taxes come from replaying it.
//
// URL parameter: orders=K opens the figure at the K-th order.

import * as M from '../js/model.js';
import { fmt } from '../js/vocab.js';
import { scene } from '../js/scene.js';

const N = 21;
const W = 400;
const H = 132;
const X = (i) => 24 + 44 * i;

/** Every order of the set that plays a legal game of n to its end, the recorded order first. */
export function legalOrders(n, set, recorded) {
  const found = [];
  (function search(state, left, path) {
    if (left.length === 0) {
      if (M.isOver(state)) found.push(path);
      return;
    }
    const legal = new Set(M.legalMoves(state));
    for (const m of left) if (legal.has(m)) search(M.play(state, m), left.filter((x) => x !== m), [...path, m]);
  })(M.newGame(n), [...set].sort((a, b) => b - a), []);
  const key = (o) => o.join(',');
  return [recorded, ...found.filter((o) => key(o) !== key(recorded))];
}

export function mountOrders(host, { small }) {
  const game = small.find((g) => g.n === N);
  const orders = legalOrders(N, game.moves, game.moves);
  const plays = orders.map((o) => M.replay(N, o));
  const pays = plays.map((p) => new Map(p.log.filter((e) => e.move !== undefined).map((e) => [e.move, e.tax])));
  // Each move takes one tax here, and the same one in every order: so the pair moves as one piece.
  const samePay = pays.every((m) => game.moves.every((x) => m.get(x).length === 1 && m.get(x)[0] === pays[0].get(x)[0]));
  const scores = new Set(plays.map((p) => M.score(p)));
  const payOf = pays[0];

  const s = scene(host.querySelector('svg'), { width: W, height: H, className: 'is-dense' });
  for (const m of game.moves) {
    const t = payOf.get(m)[0];
    s.number(m, m);
    s.number(t, t);
    s.line(`${m}-${t}`, m, t, 'pay');
  }
  const looks = (k) => {
    const v = {};
    orders[k].forEach((m, i) => {
      const t = payOf.get(m)[0];
      v[m] = { shape: 1, player: 1, x: X(i), y: 32, scale: 0.98 };
      v[t] = { shape: 0, taxman: 1, x: X(i), y: 100, scale: 0.98 };
      v[`${m}-${t}`] = {};
    });
    return v;
  };

  const out = host.querySelector('output');
  const status = host.querySelector('.orders-status');
  let k = 0;
  const factorial = (x) => (x <= 1 ? 1 : x * factorial(x - 1));
  function show(next, animate = true) {
    k = (next + orders.length) % orders.length;
    out.textContent = `order ${k + 1} of ${orders.length}`;
    const opening = orders.every((o) => o[0] === orders[0][0]) ? ` Every one opens with ${orders[0][0]}.` : '';
    // The drawing shows the order; a screen reader hears it.
    status.innerHTML = `<span class="sr-only">${orders[k].join(', ')}. </span>`
      + "The player's moves are blue, and the taxman's tax orange.  "
      + `Of the ${fmt(factorial(game.moves.length))} orders of these ${game.moves.length} numbers, `
      + `${orders.length} are legal games, `
      + `and every one scores ${[...scores].join(' or ')}${samePay ? ', with each move taking the same tax' : ''}.${opening}`;
    const v = looks(k);
    if (animate) orders[k].forEach((m, i) => {
      v[m].at = [0.04 * i, 0.04 * i + 0.62];
      v[payOf.get(m)[0]].at = [0.04 * i + 0.05, 0.04 * i + 0.67];
    });
    s.morph(v, { duration: animate ? 1000 : 0 });
    if (animate) {
      const url = new URL(location.href);
      url.searchParams.set('orders', String(k + 1));
      history.replaceState(null, '', url);
    }
  }
  host.querySelector('[data-act="prev"]').addEventListener('click', () => show(k - 1));
  host.querySelector('[data-act="next"]').addEventListener('click', () => show(k + 1));
  const start = Number(new URLSearchParams(location.search).get('orders'));
  show(Number.isInteger(start) && start >= 1 && start <= orders.length ? start - 1 : 0, false);
  return { highlight: (n, on) => s.highlight(n, on), count: orders.length };
}
