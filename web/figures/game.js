// Play the game: the pot as a tray of sockets, and the two trays that fill as the reader plays.  The numbers are
// raised tokens, since the reader takes them: a coin for a number above n/2, a tile for the rest.
//
// The game itself, its controls and its caption are js/play.js's; this file draws its board.  A legal number stands
// in its socket; one that is not lies flat and refuses to move.  A tile taken as a move turns over and lands in the
// reader's tray as a coin.
//
// URL parameter: game=N or game=N:m1,m2,... opens game N with those moves played.

import { mountPlay, shapeOf } from '../js/play.js';
import { glide, makePiece, reserveRows, traysHTML } from '../js/trays.js';
import { reducedMotion, restyle } from '../js/vocab.js';

export function mountGame(host, { small, n = 10, minN = 2, maxN = 30, param = 'game', you = 'You' }) {
  return mountPlay(host, { small, n, minN, maxN, param, you, board: trays });
}

// The board of the pot and the two trays (js/trays.js).
function trays({ you, take }) {
  let host;
  let potGrid;
  let playerSlots;
  let taxmanSlots;
  let n;
  let pieces = new Map();
  let sockets = new Map();
  const $ = (sel) => host.querySelector(sel);

  function build(newN) {
    n = newN;
    potGrid.innerHTML = '';
    playerSlots.innerHTML = '';
    taxmanSlots.innerHTML = '';
    pieces = new Map();
    sockets = new Map();
    for (let x = 1; x <= n; x++) {
      const socket = document.createElement('div');
      socket.className = 'socket';
      const piece = makePiece(x, shapeOf(n, x));
      piece.addEventListener('click', () => take(x));
      socket.appendChild(piece);
      potGrid.appendChild(socket);
      pieces.set(x, piece);
      sockets.set(x, socket);
    }
  }

  // Put every piece where the state says, and slide the ones that moved.  order lists the pieces that move in this
  // step, in the order they should start.
  function show(state, { legal, order = [] }) {
    const before = new Map();
    for (const [x, p] of pieces) before.set(x, p.getBoundingClientRect());
    for (const [x, p] of pieces) {
      let where;
      let lookName;
      let shape = shapeOf(n, x);
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
      const delay = k === 0 ? 0 : 170 + 120 * (k - 1);
      glide(p, { from: before.get(x), delay, flips: was.split('/')[0] !== shape, looks });
    }
    $('.pot-sum').textContent = state.pot.size ? `${state.pot.size} left` : 'empty';
  }

  function refuse(x) {
    const p = pieces.get(x);
    p.classList.remove('is-refused');
    void p.offsetWidth;
    p.classList.add('is-refused');
  }

  function reserve() {
    reserveRows(host, taxmanSlots, n);
  }

  return {
    html: `
    <div class="tray-box pot-box">
      <header><span>The pot</span><span class="pot-sum"></span></header>
      <div class="pot-grid" role="group" aria-label="The pot"></div>
    </div>
    ${traysHTML(you)}`,
    mount(h) {
      host = h;
      potGrid = $('.pot-grid');
      playerSlots = $('.player .slots');
      taxmanSlots = $('.taxman .slots');
    },
    build,
    show,
    refuse,
    reserve,
    // Arrive without sliding: the opening position is not news.
    settle() {
      for (const p of pieces.values()) for (const a of p.getAnimations({ subtree: true })) a.finish();
    },
  };
}
