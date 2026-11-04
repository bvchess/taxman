// The game on the maximal-factor graph: the game of figures/game.js, on a board that draws every number joined to its
// maximal factors, so that the reader sees what a move costs before making it.  The game itself, its controls and its
// caption are js/play.js's; this file draws its board.
//
// The layout is the wiki's walkthrough of game 21: one column a layer, Ω(x), the highest layer at the left and layer
// 0, the number 1, at the right; in each column the numbers ascend from the top.  The rules, the same for every n and
// width:
// * the columns stand evenly spaced, at the pitch that fits the deepest layer of the largest game across the width,
//   so the spacing does not change with n, and the graph is centered;
// * the rows are evenly spaced and top-aligned, so the small numbers, which most lines lead to, sit level along the
//   top, and the lines mostly run level or rise to the right;
// * 1 stands level with the middle of the primes, which all lead to it;
// * a line leaves its number level, to the right, and reaches its factor level, from the left, curving between the
//   columns, so that no line crosses a number;
// * the tokens grow with the width, from their plain size on a phone, where game 30 still fits at 390 px, to a
//   quarter larger.
//
// The marks are the site's: a number the reader may take is a raised token standing up, a coin for a number above
// n/2 and a tile for the rest; one that cannot be taken lies flat.  A taken number lies flat in its place, in the
// player's color or the taxman's, and a tile taken as a move turns over as a coin.  A line that leads to a number no
// longer in the pot fades.  Pointing at or focusing a number that can be taken puts a halo on its tax, everything
// still in the pot that it reaches by its lines, and on the lines it follows.  On a touch screen the first tap shows
// the tax and the second takes the number.
//
// URL parameter: graph=N or graph=N:m1,m2,... opens game N with those moves played.

import * as M from '../js/model.js';
import { mountPlay, shapeOf } from '../js/play.js';
import { scene } from '../js/scene.js';
import { MOTION, NS, SIZE, listWords, look } from '../js/vocab.js';

export function mountGraphGame(host, { small, n = 21 }) {
  let view;
  const game = mountPlay(host, { small, n, param: 'graph', board: (context) => (view = graphBoard(context)) });
  return { ...game, highlight: (x, on) => view.highlight(x, on) };
}

const ROW = 38; // the distance between rows, in the units of a token, which is 30 across as a coin
const POINT_MS = 150; // the halo answering the pointer, as --ui does

// Where every number of game n stands on a board width pixels wide.  deepest is the largest layer of the largest game,
// which sets the pitch of the columns.
function layout(n, width, deepest) {
  const scale = Math.min(1.25, Math.max(1, width / 330));
  const half = SIZE.circle * scale;
  const pad = half + 4;
  const pitch = (width - 2 * pad) / deepest;
  const row = ROW * scale;
  const top = half + 8;
  const columns = [];
  for (let x = 1; x <= n; x++) (columns[M.omega(x)] ??= []).push(x);
  const last = columns.length - 1;
  const left = width / 2 - (pitch * last) / 2;
  const at = new Map();
  columns.forEach((column, k) => {
    column.forEach((x, i) => at.set(x, { x: left + pitch * (last - k), y: top + row * i }));
  });
  if (columns[1]) at.get(1).y = top + (row * (columns[1].length - 1)) / 2;
  const rows = Math.max(...columns.map((c) => c.length));
  const height = top + row * (rows - 1) + (SIZE.circle + SIZE.depth) * scale + 10;
  // The numbers in reading order, a column at a time from the left, for the tab order.
  const order = columns.slice().reverse().flat();
  return { at, scale, width, height, order };
}

function graphBoard({ you, take, maxN, playing }) {
  let host;
  let wrap;
  let s = null; // the scene
  let n = 0;
  let lay = null;
  let lines = []; // [id, number, its maximal factor]
  let state = null;
  let legal = new Set();
  let shown = ''; // the state drawn, to skip drawing it again
  let pointed = null; // the number under the pointer or the focus
  let armed = true; // a click takes the number; a first tap on a touch screen only points at it
  let motion = 0; // the id of the latest move drawn, so a halo waits for it to land
  let moving = false;
  let deferred = false;
  let deepest = 0;
  for (let x = 1; x <= maxN; x++) deepest = Math.max(deepest, M.omega(x));

  // The tax of the number pointed at, if it can be taken now: the numbers still in the pot it reaches by its lines.
  function lit() {
    if (pointed === null || !legal.has(pointed) || playing()) return new Set();
    return new Set([pointed, ...M.properDivisors(pointed).filter((d) => state.pot.has(d))]);
  }

  function looks() {
    const halo = lit();
    const v = {};
    for (let x = 1; x <= n; x++) {
      const base = { ...lay.at.get(x), scale: lay.scale, shape: shapeOf(n, x) === 'move' ? 1 : 0 };
      if (state.pot.has(x)) {
        v[x] = { ...base, ...(legal.has(x) ? {} : look('lying')), halo: halo.has(x) && x !== pointed ? 1 : 0 };
      } else if (state.player.includes(x)) v[x] = { ...base, shape: 1, player: 1, depth: 0 };
      else v[x] = { ...base, taxman: 1, depth: 0 };
    }
    for (const [id, x, f] of lines) {
      v[id] = state.pot.has(f)
        ? { bend: 1, halo: halo.has(x) && halo.has(f) ? 1 : 0 }
        : { bend: 1, faded: 1, opacity: 0.75 };
    }
    // The faded lines lie under the others, and the lines of a tax over them.
    const rank = (x, f) => (!state.pot.has(f) ? 0 : halo.has(x) && halo.has(f) ? 2 : 1);
    for (const k of [0, 1, 2]) {
      for (const [id, x, f] of lines) if (rank(x, f) === k) s.layer.line.appendChild(s.node(id));
    }
    return v;
  }

  function draw(duration, windows = null) {
    const v = looks();
    if (windows) for (const [id, w] of Object.entries(windows)) if (v[id]) v[id].at = w;
    const id = ++motion;
    moving = duration > POINT_MS;
    s.morph(v, { duration }).then(() => {
      if (id !== motion) return;
      moving = false;
      if (deferred) {
        deferred = false;
        draw(POINT_MS);
      }
    });
  }

  function point(x) {
    if (pointed === x) return;
    const was = pointed;
    pointed = x;
    for (const y of [was, x]) s.node(y)?.classList.toggle('is-pointed', y !== null && y === pointed);
    if (moving) deferred = true;
    else draw(POINT_MS);
  }

  function label(x) {
    if (state.pot.has(x)) {
      if (!legal.has(x)) return `${x}, cannot be taken`;
      return `${x}: the taxman would take ${listWords(M.properDivisors(x).filter((d) => state.pot.has(d)))}`;
    }
    if (state.player.includes(x)) return `${x}, ${you === 'You' ? 'yours' : 'the player’s'}`;
    return `${x}, the taxman’s`;
  }

  function attributes() {
    for (let x = 1; x <= n; x++) {
      const node = s.node(x);
      const can = legal.has(x);
      node.setAttribute('tabindex', can ? '0' : '-1');
      node.setAttribute('aria-disabled', String(!can));
      node.setAttribute('aria-label', label(x));
      node.classList.toggle('is-takeable', can);
      node.classList.toggle('is-stuck', state.pot.has(x) && !can);
    }
  }

  function size() {
    s.svg.setAttribute('viewBox', `0 0 ${lay.width} ${lay.height}`);
    s.svg.style.width = `${lay.width}px`;
    s.svg.style.height = `${lay.height}px`;
  }

  function build(newN) {
    const width = wrap.getBoundingClientRect().width;
    if (s && newN === n && Math.abs(width - lay.width) < 0.5) return;
    n = newN;
    lay = layout(n, width, deepest);
    pointed = null;
    shown = '';
    const svg = document.createElementNS(NS, 'svg');
    svg.setAttribute('role', 'group');
    wrap.replaceChildren(svg);
    s = scene(svg, {
      width: lay.width, height: lay.height, rendering: 'raised',
      label: `The maximal-factor graph of game ${n}, to play on: the numbers 1 to ${n} in columns by layer, `
        + 'each joined by a line to its maximal factors in the column to its right.',
    });
    size();
    lines = [];
    for (const x of lay.order) {
      s.number(x, x);
      const node = s.node(x);
      node.setAttribute('role', 'button');
      node.addEventListener('pointerenter', (e) => e.pointerType !== 'touch' && point(x));
      node.addEventListener('pointerleave', (e) => e.pointerType !== 'touch' && pointed === x && point(null));
      node.addEventListener('focus', () => point(x));
      node.addEventListener('blur', () => pointed === x && point(null));
      node.addEventListener('pointerdown', (e) => (armed = e.pointerType === 'mouse' || pointed === x));
      node.addEventListener('click', () => {
        const first = !armed && legal.has(x) && !playing();
        armed = true;
        if (first) point(x);
        else take(x);
      });
      node.addEventListener('keydown', (e) => {
        if (e.key !== 'Enter' && e.key !== ' ') return;
        e.preventDefault();
        take(x);
      });
    }
    for (let x = 2; x <= n; x++) {
      for (const f of M.maximalFactors(x)) {
        const id = `${x}>${f}`;
        s.line(id, x, f, 'thin');
        lines.push([id, x, f]);
      }
    }
  }

  // Draw the state.  A move lands as in the main game: the move first, then its tax and the rest, one after another.
  function show(next, { legal: open, order = [] }) {
    state = next;
    legal = open;
    attributes();
    const key = `${[...state.pot]}|${state.player}|${state.taxman}`;
    if (key === shown && order.length === 0) return;
    shown = key;
    if (order.length === 0) {
      draw(0);
      return;
    }
    const start = order.map((_, k) => (k === 0 ? 0 : 170 + 120 * (k - 1)));
    const total = start[start.length - 1] + MOTION.move;
    const windows = {};
    const span = (t0) => [t0 / total, (t0 + MOTION.move) / total];
    order.forEach((x, k) => (windows[x] = span(start[k])));
    // The lines fade as the number they lead to leaves, and a number left with no line lies down as the last lands.
    for (const [id, , f] of lines) if (order.includes(f)) windows[id] = windows[f];
    for (let x = 1; x <= n; x++) if (!order.includes(x)) windows[x] = span(start[start.length - 1]);
    draw(total, windows);
  }

  function refuse(x) {
    const node = s.node(x);
    node.classList.remove('is-refused');
    void node.getBoundingClientRect();
    node.classList.add('is-refused');
  }

  // A new width lays the board out again; its height is fixed for n and the width.
  function reserve() {
    const width = wrap.getBoundingClientRect().width;
    if (Math.abs(width - lay.width) < 0.5) return;
    lay = layout(n, width, deepest);
    size();
    draw(0);
  }

  return {
    html: `
    <div class="tray-box graph-board">
      <header>
        <span class="player"><span class="who">${you}</span><span class="score">0</span></span>
        <span class="taxman"><span class="who">Taxman</span><span class="score">0</span></span>
      </header>
      <div class="graph-wrap"></div>
    </div>`,
    mount(h) {
      host = h;
      wrap = host.querySelector('.graph-wrap');
    },
    build,
    show,
    refuse,
    reserve,
    settle() {
      s.stop();
      draw(0);
    },
    highlight: (x, on) => s?.highlight(x, on),
  };
}
