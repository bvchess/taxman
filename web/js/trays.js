// The two trays of the game, the player's and the taxman's, and the motion of a piece into and out of them.  Both
// boards of the game use them: figures/game.js, whose pot is a tray of sockets, and figures/graph-game.js, whose pot
// is the maximal-factor graph.  A piece is a token in an HTML button, so that it can travel from anywhere on the
// page, and a tray holds the pieces of the numbers taken.
//
// The marks are the site's system: a circle is a move the player could take, a square a number that will go to the
// taxman, and a number with no proper divisor left in the pot can never be a move, so its circle turns into a square.
// In the pot a circle the player can take now is a raised coin, which the reader presses; a square is flat.  Taken
// numbers lie flat in the trays: the player's as blue circles, the taxman's as orange squares.  potLook() and TRAY_LOOK
// give the looks, which both boards share.
//
// glide() is the motion of a take, and of a take back, which the two boards share: the piece is lifted on the way and
// lands in its tray as a flat mark, or, going back, returns to its place in the pot.  A piece travels
// between its place in its tray and a box on the page, by a transform that moves and scales it about its center, so
// that a piece coming from a token of another size grows or shrinks on the way.  A piece sent somewhere while it is
// still traveling leaves its first journey at once.  startOf() and returnDelays() say when the pieces of a step set
// out, going and coming back.

import { MOTION, numberSVG, restyle } from './vocab.js';

const EASE = MOTION.settleCSS; // out, with a little overshoot: a raised token is set down

const FLAT = { depth: 0 };

/** The look of a number in the pot: a raised coin if the player can take it now, else a flat square. */
export const potLook = (canTake) => (canTake ? ['move'] : ['factor', FLAT]);

/** The look of a taken number in its tray, flat: the player's a blue circle, the taxman's an orange square. */
export const TRAY_LOOK = { player: ['move', FLAT, 'player'], taxman: ['factor', FLAT, 'taxman'] };

/** When the k-th number of a step sets out, in milliseconds: the move (k = 0) at once, the rest one after another. */
export const startOf = (k) => (k === 0 ? 0 : 170 + 120 * (k - 1));

/** When each of the pieces going back to the pot sets out, the last of its step to leave being the first to return.
 *  back lists the pieces, each as { step, place }: the step that took it and its place in that step (0 for the move).
 *  Returns their delays in milliseconds, in the same order. */
export function returnDelays(back) {
  const last = new Map();
  for (const { step, place } of back) last.set(step, Math.max(last.get(step) ?? 0, place));
  return back.map(({ step, place }) => startOf(last.get(step)) - startOf(place));
}

/** The markup of the two trays, with the scores in .player .score and .taxman .score and the pieces in .slots. */
export function traysHTML(you) {
  const tray = (kind, who, label) => `
      <div class="tray-box ${kind}">
        <header><span class="who">${who}</span><span class="score">0</span></header>
        <div class="slots" role="group" aria-label="${label}"></div>
      </div>`;
  return `
    <div class="trays">${tray('player', you, you === 'You' ? 'Your numbers' : 'The player’s numbers')}${
  tray('taxman', 'Taxman', 'The taxman’s numbers')}
    </div>`;
}

/** The piece of x: a button holding its token, drawn with looks. */
export function makePiece(x, ...looks) {
  const piece = document.createElement('button');
  piece.type = 'button';
  piece.className = 'piece';
  piece.dataset.n = x;
  piece.appendChild(numberSVG(x, { rendering: 'raised', looks, size: 50, pad: 3 }));
  return piece;
}

/** Keep the tray rows of the fullest tray, in which one tray may hold every number of game n but one, so that nothing
 *  below the trays moves while the game is played. */
export function reserveRows(host, slots, n) {
  const probe = slots.appendChild(document.createElement('span'));
  probe.className = 'piece';
  const width = probe.getBoundingClientRect().width;
  probe.remove();
  const perRow = Math.max(1, Math.floor(slots.clientWidth / width));
  host.style.setProperty('--tray-rows', Math.ceil((n - 1) / perRow));
}

/** The box that piece would fill if its token, drawn unit pixels to a figure unit, were centered at (x, y) of the
 *  screen: where a piece sets out from, or arrives at, a token of a figure. */
export function boxAround(piece, x, y, unit) {
  const box = piece.getBoundingClientRect();
  const face = piece.querySelector('svg');
  const r = face.getBoundingClientRect();
  const view = face.viewBox.baseVal;
  const k = unit / (r.width / view.width);
  const cx = r.left - view.x * (r.width / view.width);
  const cy = r.top - view.y * (r.width / view.width);
  return { left: x - (cx - box.left) * k, top: y - (cy - box.top) * k, width: box.width * k, height: box.height * k };
}

// The transform that makes the piece, which stands at home, appear in the box.
function transformTo(box, home) {
  const dx = box.left + box.width / 2 - (home.left + home.width / 2);
  const dy = box.top + box.height / 2 - (home.top + home.height / 2);
  const k = box.width / home.width;
  return `translate(${dx}px, ${dy}px)${Math.abs(k - 1) > 1e-6 ? ` scale(${k})` : ''}`;
}

/** Slide a piece to its place from the box it was in, or, with to, from its place to that box; delay is when it sets
 *  out.  With flips the piece turns over on the way.  When it is halfway, the piece is drawn with looks.  First stops
 *  any motion of the piece in progress and the restyle it had scheduled.  Returns the travel. */
export function glide(piece, { from = null, to = null, delay = 0, flips = false, looks }) {
  const svg = piece.querySelector('svg');
  // A journey in progress ends here: neither its motion nor its restyle may arrive after this one is begun.
  for (const a of [...piece.getAnimations(), ...svg.getAnimations()]) a.cancel();
  clearTimeout(piece._restyle);
  const home = piece.getBoundingClientRect();
  piece.classList.add('is-moving');
  const travel = piece.animate(
    [{ transform: from ? transformTo(from, home) : 'none' }, { transform: to ? transformTo(to, home) : 'none' }],
    { duration: MOTION.move, delay, easing: EASE, fill: to ? 'both' : 'backwards' },
  );
  if (!to) travel.onfinish = () => piece.classList.remove('is-moving');
  // Lifted on the way, and a piece that flips turns over halfway.
  const flip = [{ transform: 'scale(1,1)' }, { transform: 'translateY(-10px) scale(0.05,1.08)', offset: 0.45 },
    { transform: 'scale(1,1)' }];
  const lift = [{ transform: 'scale(1)' }, { transform: 'translateY(-8px) scale(1.08)', offset: 0.4 },
    { transform: 'scale(1)' }];
  svg.animate(
    flips ? flip : lift,
    { duration: MOTION.move, delay, easing: 'ease-in-out', fill: 'backwards' },
  );
  piece._restyle = setTimeout(() => restyle(svg, ...looks), delay + (flips ? 280 : 200));
  return travel;
}
