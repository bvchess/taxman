// The game the reader plays, apart from its board: the rules of js/model.js, the controls, and the caption.
//
// A number is taken only if a divisor of it is still in the pot, and the taxman takes every such divisor; a number
// that cannot be taken refuses, and the caption says why.  The controls are − n +, Take back, Start over and the blue
// "Play an optimal game", which plays the recorded game of n from data/small.json.  The caption opens with the
// figure's opening line, says what each move took, and at the end compares the score with opt(n).
//
// A figure gives the game its board, which draws the state and calls take(x) when the reader takes x: figures/game.js
// draws the pot of sockets and the two trays, figures/graph-game.js the maximal-factor graph as the pot, over the same
// two trays (js/trays.js).  board(context) makes the board, where context is { you, take, minN, maxN, playing }, and
// playing() says whether an optimal game is being played.
// The board has
//   html           its markup, which goes between the controls and the caption, with the scores in .player .score and
//                  .taxman .score
//   mount(host)    called once the markup is in the page
//   build(n)       draw the board of game n
//   show(state, { legal, order })   draw a state: legal is the set of legal moves, and order lists the numbers that
//                  move in this step, in the order they should start (the move, its tax, then the rest); a number
//                  that the state brings back from a tray to the pot travels back, whatever the order, and every other
//                  number is drawn at once as the state says
//   refuse(x)      x cannot be taken: shake it
//   reserve(n)     keep the room that the fullest state of game n needs at the present width
//   settle()       end every motion, for a game that opens with moves played
//
// Nothing outside the game moves while it is played, so the game keeps one height for the current n and width: the
// caption has room for its longest message, and the board for its fullest state.  Both are measured when n or the
// width changes, never during play.
//
// URL parameter: param=N or param=N:m1,m2,... opens game N with those moves played.

import * as M from './model.js';
import { listWords, reducedMotion } from './vocab.js';

const sum = (xs) => xs.reduce((a, b) => a + b, 0);
const range = (a, b) => Array.from({ length: Math.max(0, b - a + 1) }, (_, i) => a + i);

export function mountPlay(host, {
  small, board, n: startN = 10, minN = 2, maxN = 30, param = 'game', you = 'You',
  opening = 'Take a number. The taxman takes all of its divisors still in the pot.',
}) {
  const best = new Map(small.map((g) => [g.n, g]));
  let n = startN;
  let state = null;
  let history = [];
  let playing = 0; // the id of a running optimal game, 0 if none

  const view = board({ you, take: (x) => take(x), minN, maxN, playing: () => playing !== 0 });
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
    </div>${view.html}
    <p class="game-status" aria-live="polite"></p>`;
  const $ = (sel) => host.querySelector(sel);
  const status = $('.game-status');
  const out = $('output');
  view.mount(host);

  function build() {
    view.build(n);
    out.textContent = `n = ${n}`;
    $('[data-act="fewer"]').disabled = n <= minN;
    $('[data-act="more"]').disabled = n >= maxN;
  }

  // Draw the state; order lists the numbers that move in this step, in the order they should start.
  function place(order = []) {
    view.show(state, { legal: new Set(state.pot.size ? M.legalMoves(state) : []), order });
    $('.player .score').textContent = sum(state.player);
    $('.taxman .score').textContent = sum(state.taxman);
    $('[data-act="back"]').disabled = history.length === 0 || playing !== 0;
  }

  // The caption has room for its longest message, and the board for its fullest state.  Called when n or the width
  // changes, never during play.
  function reserve() {
    // The longest messages: the divisors that make the longest tax; at the end, the rest is a set with no divisor
    // pair, so at most ceil(n/2) numbers, longest as the largest ones; the player holds at most that many too.
    const divisors = (x) => range(1, Math.floor(x / 2)).filter((d) => x % d === 0);
    const taxOf = range(2, n).map((x) => [x, divisors(x)]).sort((a, b) => b[1].length - a[1].length)[0];
    const [x, tax] = taxOf ?? [n, []];
    const top = range(n - Math.ceil(n / 2) + 1, n);
    const rest = top.filter((t) => t !== x && !tax.includes(t));
    const messages = [
      opening, refusal(1), refusal(n), tookWords(x, tax), takenBack(top),
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
    view.reserve(n);
  }

  function setStatus(html) {
    status.innerHTML = html;
  }

  function verdict(mine) {
    const opt = best.get(n)?.score;
    if (opt === undefined) return `You scored ${mine}.`;
    if (mine === opt) return `<strong>${mine} is the best score in game ${n}</strong>: an optimal game.`;
    return `The best score in game ${n} is ${opt}, so you were ${opt - mine} short.`
      + ' <strong>Play an optimal game</strong> to see one.';
  }

  // The caption's messages, built in one place so that reserve() can measure the longest of them.
  const tookWords = (x, tax) => `${you === 'You' ? 'You' : 'The player'} took ${x}; the taxman took ${listWords(tax)}.`;
  const endWords = (rest, mine, theirs) => (rest.length
    ? ` No move is left, so the taxman takes the rest: ${listWords(rest)}.`
    : ' No move is left, and the pot is empty.') + ` ${you} ${mine}, taxman ${theirs}. ${verdict(mine)}`;
  const refusal = (x) => (x === 1
    ? '1 can’t be taken: it has no divisor below it, so the taxman would get nothing.'
    : `${x} can’t be taken: none of its divisors is left in the pot, so the taxman would get nothing.`);
  const takenBack = (mine) => `Taken back. ${mine.length ? `You have ${listWords(mine)}.` : 'Take a number.'}`;

  function take(x, { quiet = false } = {}) {
    if (!state.pot.has(x)) return false;
    if (playing && !quiet) return false;
    const legal = M.legalMoves(state);
    if (!legal.includes(x)) {
      view.refuse(x);
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
    setStatus(opening);
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
  // Arrive without moving: the opening position is not news.
  view.settle();
  return { reset, playBest };
}
