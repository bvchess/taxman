// The two trays of the game, the player's and the taxman's, and the motion of a piece into and out of them.  Both
// boards of the game use them: figures/game.js, whose pot is a tray of sockets, and figures/graph-game.js, whose pot
// is the maximal-factor graph.  A piece is a raised token in an HTML button, so that it can travel from anywhere on the
// page, and a tray holds the pieces of the numbers taken.
//
// glide() is the motion of a take, which the two boards share: the piece is lifted on the way, and a tile taken as a
// move turns over halfway and lands as a coin.  A piece travels between its place in its tray and a box on the page,
// by a transform that moves and scales it about its center, so that a piece coming from a token of another size
// grows or shrinks on the way.

import { MOTION, numberSVG, restyle } from './vocab.js';

const EASE = MOTION.settleCSS; // out, with a little overshoot: a raised token is set down

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

/** The piece of x: a button holding its raised token, drawn with looks. */
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
 *  out.  With flips the tile turns over on the way.  When it is halfway, the piece is drawn with looks.  Returns the
 *  travel. */
export function glide(piece, { from = null, to = null, delay = 0, flips = false, looks }) {
  const svg = piece.querySelector('svg');
  const home = piece.getBoundingClientRect();
  piece.classList.add('is-moving');
  const travel = piece.animate(
    [{ transform: from ? transformTo(from, home) : 'none' }, { transform: to ? transformTo(to, home) : 'none' }],
    { duration: MOTION.move, delay, easing: EASE, fill: to ? 'both' : 'backwards' },
  );
  if (!to) travel.onfinish = () => piece.classList.remove('is-moving');
  // Lifted on the way, and a tile taken as a move flips over halfway.
  const flip = [{ transform: 'scale(1,1)' }, { transform: 'translateY(-10px) scale(0.05,1.08)', offset: 0.45 },
    { transform: 'scale(1,1)' }];
  const lift = [{ transform: 'scale(1)' }, { transform: 'translateY(-8px) scale(1.08)', offset: 0.4 },
    { transform: 'scale(1)' }];
  svg.animate(
    flips ? flip : lift,
    { duration: MOTION.move, delay, easing: 'ease-in-out', fill: 'backwards' },
  );
  setTimeout(() => restyle(svg, ...looks), delay + (flips ? 280 : 200));
  return travel;
}
