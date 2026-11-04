// Play the game: the pot as a tray of sockets, and the two trays that fill as the reader plays.  The numbers are
// raised tokens, since the reader takes them: a coin for a number above n/2, a tile for the rest.
//
// The rules are js/model.js's (legalMoves, play, finish): a number is taken only if a divisor of it is still in
// the pot, and the taxman takes every such divisor.  A legal number stands in its socket; one that is not lies flat
// and refuses to move.  A tile taken as a move turns over and lands in the reader's tray as a coin.  The button
// "Play an optimal game" plays the recorded game of n from data/small.json.
//
// URL parameter: game=N or game=N:m1,m2,... opens game N with those moves played.

import * as M from '../js/model.js';
import { MOTION, listWords, numberSVG, reducedMotion, restyle } from '../js/vocab.js';

const EASE = MOTION.settleCSS; // out, with a little overshoot: a raised token is set down

export function mountGame(host, { small, n: startN = 10, minN = 2, maxN = 30, param = 'game', you = 'You' }) {
  const best = new Map(small.map((g) => [g.n, g]));
  host.classList.add('game');
  host.innerHTML = `
    <div class="game-controls">
      <span class="stepper" role="group" aria-label="Size of the board">
        <button type="button" class="btn round" data-act="fewer" aria-label="One number fewer">&minus;</button>
        <output aria-live="polite"></output>
        <button type="button" class="btn round" data-act="more" aria-label="One number more">+</button>
      </span>
      <button type="button" class="btn" data-act="back">Take back</button>
      <button type="button" class="btn" data-act="reset">Start over</button>
      <button type="button" class="btn primary" data-act="best">Play an optimal game</button>
    </div>
    <div class="tray-box pot-box">
      <header><span>The pot</span><span class="pot-sum"></span></header>
      <div class="pot-grid" role="group" aria-label="The pot"></div>
    </div>
    <div class="trays">
      <div class="tray-box player"><header><span class="who">${you}</span><span class="score">0</span></header><div class="slots" role="group" aria-label="${you === 'You' ? 'Your numbers' : 'The player’s numbers'}"></div></div>
      <div class="tray-box taxman"><header><span class="who">Taxman</span><span class="score">0</span></header><div class="slots" role="group" aria-label="The taxman’s numbers"></div></div>
    </div>
    <p class="game-status" aria-live="polite"></p>`;
  const $ = (sel) => host.querySelector(sel);
  const potGrid = $('.pot-grid');
  const playerSlots = $('.player .slots');
  const taxmanSlots = $('.taxman .slots');
  const status = $('.game-status');
  const out = $('output');

  let n = startN;
  let state = null;
  let history = [];
  let pieces = new Map();
  let sockets = new Map();
  let playing = 0; // the id of a running optimal game, 0 if none

  const shapeOf = (x) => (x > n / 2 ? 'move' : 'factor');

  function build() {
    potGrid.innerHTML = '';
    playerSlots.innerHTML = '';
    taxmanSlots.innerHTML = '';
    pieces = new Map();
    sockets = new Map();
    for (let x = 1; x <= n; x++) {
      const socket = document.createElement('div');
      socket.className = 'socket';
      const piece = document.createElement('button');
      piece.type = 'button';
      piece.className = 'piece';
      piece.dataset.n = x;
      piece.appendChild(numberSVG(x, { rendering: 'raised', looks: [shapeOf(x)], size: 50, pad: 3 }));
      piece.addEventListener('click', () => take(x));
      socket.appendChild(piece);
      potGrid.appendChild(socket);
      pieces.set(x, piece);
      sockets.set(x, socket);
    }
    out.textContent = `n = ${n}`;
    $('[data-act="fewer"]').disabled = n <= minN;
    $('[data-act="more"]').disabled = n >= maxN;
  }

  // Put every piece where the state says, and slide the ones that moved.  order lists the pieces that move in this
  // step, in the order they should start.
  function place(order = []) {
    const before = new Map();
    for (const [x, p] of pieces) before.set(x, p.getBoundingClientRect());
    const legal = new Set(state.pot.size ? M.legalMoves(state) : []);
    for (const [x, p] of pieces) {
      let where;
      let lookName;
      let shape = shapeOf(x);
      if (state.pot.has(x)) {
        where = sockets.get(x);
        lookName = legal.has(x) ? 'standing' : 'lying';
      } else if (state.player.includes(x)) {
        where = playerSlots;
        lookName = 'player';
        shape = 'move';
      } else {
        where = taxmanSlots;
        lookName = 'taxman';
      }
      p.classList.toggle('is-legal', legal.has(x));
      p.classList.toggle('is-stuck', state.pot.has(x) && !legal.has(x));
      p.disabled = !state.pot.has(x);
      p.setAttribute('aria-label', state.pot.has(x)
        ? `${x}${legal.has(x) ? '' : ', cannot be taken'}`
        : `${x}, ${lookName === 'player' ? (you === 'You' ? 'yours' : 'the player’s') : 'the taxman’s'}`);
      p._next = { where, lookName, shape };
    }
    // Move in the DOM: the trays in the order of the state's lists, so they fill left to right.
    for (const [x, p] of pieces) if (p._next.where === sockets.get(x)) p._next.where.appendChild(p);
    for (const x of state.player) playerSlots.appendChild(pieces.get(x));
    for (const x of state.taxman) taxmanSlots.appendChild(pieces.get(x));
    const quick = reducedMotion();
    for (const [x, p] of pieces) {
      const svg = p.querySelector('svg');
      const { lookName, shape } = p._next;
      const looks = lookName === 'standing' ? [shape] : [shape, lookName];
      const k = order.indexOf(x);
      const was = p._shown;
      p._shown = `${shape}/${lookName}`;
      if (quick || k < 0 || was === undefined) {
        restyle(svg, ...looks);
        continue;
      }
      const a = before.get(x);
      const b = p.getBoundingClientRect();
      const delay = k === 0 ? 0 : 170 + 120 * (k - 1);
      p.classList.add('is-moving');
      const travel = p.animate(
        [{ transform: `translate(${a.left - b.left}px, ${a.top - b.top}px)` }, { transform: 'none' }],
        { duration: MOTION.move, delay, easing: EASE, fill: 'backwards' },
      );
      travel.onfinish = () => p.classList.remove('is-moving');
      // Lifted on the way, and a tile taken as a move flips over halfway.
      const flips = was.split('/')[0] !== shape;
      svg.animate(
        flips
          ? [{ transform: 'scale(1,1)' }, { transform: 'translateY(-10px) scale(0.05,1.08)', offset: 0.45 }, { transform: 'scale(1,1)' }]
          : [{ transform: 'scale(1)' }, { transform: 'translateY(-8px) scale(1.08)', offset: 0.4 }, { transform: 'scale(1)' }],
        { duration: MOTION.move, delay, easing: 'ease-in-out', fill: 'backwards' },
      );
      setTimeout(() => restyle(svg, ...looks), delay + (flips ? 280 : 200));
    }
    $('.player .score').textContent = sum(state.player);
    $('.taxman .score').textContent = sum(state.taxman);
    $('.pot-sum').textContent = state.pot.size ? `${state.pot.size} left` : 'empty';
    $('[data-act="back"]').disabled = history.length === 0 || playing !== 0;
  }

  // Nothing outside the game moves while it is played, so the game's box keeps one height for the current n and width:
  // the caption has room for its longest message, and the trays for their fullest.  Called when n or the width
  // changes, never during play.
  const sum = (xs) => xs.reduce((a, b) => a + b, 0);
  const range = (a, b) => Array.from({ length: Math.max(0, b - a + 1) }, (_, i) => a + i);
  function reserve() {
    // The longest messages: the divisors that make the longest tax; at the end, the rest is a set with no divisor
    // pair, so at most ceil(n/2) numbers, longest as the largest ones; the player holds at most that many too.
    const divisors = (x) => range(1, Math.floor(x / 2)).filter((d) => x % d === 0);
    const taxOf = range(2, n).map((x) => [x, divisors(x)]).sort((a, b) => b[1].length - a[1].length)[0];
    const [x, tax] = taxOf ?? [n, []];
    const top = range(n - Math.ceil(n / 2) + 1, n);
    const rest = top.filter((t) => t !== x && !tax.includes(t));
    const messages = [
      OPENING, refusal(1), refusal(n), tookWords(x, tax), takenBack(top),
      ...[sum(top), best.get(n)?.score ?? 0].map((mine) => tookWords(x, tax) + endWords(rest, mine, sum(range(1, n)))),
    ];
    const keep = status.innerHTML;
    status.style.minHeight = '0';
    let tall = 0;
    for (const m of messages) {
      status.innerHTML = m;
      tall = Math.max(tall, status.getBoundingClientRect().height);
    }
    status.innerHTML = keep;
    status.style.minHeight = `${tall}px`;
    // The fullest tray: the taxman may hold every number but one.
    const slot = potGrid.querySelector('.socket').getBoundingClientRect();
    const perRow = Math.max(1, Math.floor(taxmanSlots.clientWidth / slot.width));
    host.style.setProperty('--tray-rows', Math.ceil((n - 1) / perRow));
  }

  function setStatus(html) {
    status.innerHTML = html;
  }

  function verdict(mine) {
    const opt = best.get(n)?.score;
    if (opt === undefined) return `You scored ${mine}.`;
    if (mine === opt) return `<strong>${mine} is the best score for n = ${n}</strong>: an optimal game.`;
    return `The best score for n = ${n} is ${opt}, so you were ${opt - mine} short. <strong>Play an optimal game</strong> to see one.`;
  }

  // The caption's messages, built in one place so that reserve() can measure the longest of them.
  const tookWords = (x, tax) => `${you === 'You' ? 'You' : 'The player'} took ${x}; the taxman took ${listWords(tax)}.`;
  const endWords = (rest, mine, theirs) => (rest.length
    ? ` No move is left, so the taxman takes the rest: ${listWords(rest)}.`
    : ' No move is left, and the pot is empty.') + ` ${you} ${mine}, taxman ${theirs}. ${verdict(mine)}`;
  const refusal = (x) => (x === 1
    ? '1 can’t be taken: it has no divisor below it, so the taxman would get nothing.'
    : `${x} can’t be taken: none of its divisors is left in the pot, so the taxman would get nothing.`);
  const OPENING = 'Take a number. The taxman takes all of its divisors still in the pot.';
  const takenBack = (mine) => `Taken back. ${mine.length ? `You have ${listWords(mine)}.` : 'Take a number.'}`;

  function take(x, { quiet = false } = {}) {
    if (!state.pot.has(x)) return false;
    if (playing && !quiet) return false;
    const legal = M.legalMoves(state);
    if (!legal.includes(x)) {
      const p = pieces.get(x);
      p.classList.remove('is-refused');
      void p.offsetWidth;
      p.classList.add('is-refused');
      setStatus(refusal(x));
      return false;
    }
    history.push(state);
    state = M.play(state, x);
    const { tax } = state.log[state.log.length - 1];
    let words = tookWords(x, tax);
    if (M.isOver(state)) {
      const rest = [...state.pot].sort((a, b) => a - b);
      state = M.finish(state);
      words += endWords(rest, M.score(state), state.taxman.reduce((s, t) => s + t, 0));
      place([x, ...tax, ...rest]);
    } else place([x, ...tax]);
    setStatus(words);
    return true;
  }

  function reset(newN = n) {
    playing = 0;
    n = newN;
    state = M.newGame(n);
    history = [];
    build();
    reserve();
    place();
    setStatus(OPENING);
  }

  async function playBest() {
    reset(n);
    const id = Math.random();
    playing = id;
    const game = best.get(n);
    host.classList.add('is-playing');
    for (const m of game.moves) {
      await new Promise((r) => setTimeout(r, reducedMotion() ? 0 : 900));
      if (playing !== id) return;
      take(m, { quiet: true });
    }
    playing = 0;
    host.classList.remove('is-playing');
    place();
  }

  host.addEventListener('click', (e) => {
    const act = e.target.closest('[data-act]')?.dataset.act;
    if (!act) return;
    if (act === 'fewer' && n > minN) reset(n - 1);
    if (act === 'more' && n < maxN) reset(n + 1);
    if (act === 'reset') reset(n);
    if (act === 'best') playBest();
    if (act === 'back' && history.length) {
      state = history.pop();
      place();
      setStatus(takenBack(state.player));
    }
  });

  // Open where the URL says: game=12 or game=12:11,9.
  const v = param ? new URLSearchParams(location.search).get(param) : null;
  let moves = [];
  if (v) {
    const [a, b] = v.split(':');
    const want = Number(a);
    if (Number.isInteger(want) && want >= minN && want <= maxN) n = want;
    moves = (b ?? '').split(',').filter(Boolean).map(Number);
  }
  reset(n);
  // A new width, or the fonts arriving, changes how the messages wrap.
  let width = host.clientWidth;
  new ResizeObserver(() => {
    if (host.clientWidth !== width) {
      width = host.clientWidth;
      reserve();
    }
  }).observe(host);
  document.fonts?.ready.then(reserve);
  for (const m of moves) if (!take(m, { quiet: true })) break;
  // Arrive without sliding: the opening position is not news.
  for (const p of pieces.values()) for (const a of p.getAnimations({ subtree: true })) a.finish();
  return { reset, playBest };
}
