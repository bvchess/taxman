// The game on the maximal-factor graph: the game of figures/game.js, on a board whose pot is the graph of every number
// joined to its maximal factors, so that the reader sees what a move costs before making it.  The game itself, its
// controls and its caption are js/play.js's; this file draws its board: the graph, and below it the two trays of
// js/trays.js.
//
// The layout is the wiki's walkthrough of game 21: one column a layer, Ω(x), the highest layer at the left and layer
// 0, the number 1, at the right; in each column the numbers ascend from the top.  The rules, the same for every n and
// width:
// * the columns stand evenly spaced, at the pitch that fits the deepest layer of the largest game across the width,
//   so the spacing does not change with n, and the graph is centered;
// * the rows are evenly spaced and top-aligned, so the small numbers, which most lines lead to, sit level along the
//   top, and the lines mostly run level or rise to the right;
// * 1 stands level with the middle of the primes, which all lead to it;
// * a line is straight, from a number to its maximal factor in the next column, and lies under the tokens, so that
//   no line crosses a number;
// * the tokens grow with the width, from their plain size on a phone, where game 30 still fits at 390 px, to a
//   quarter larger.
//
// The marks are the site's: every number is plain, between a tile and a coin, in the pot and in the trays; a number
// the reader may take is a raised token standing up, and one that cannot be taken lies flat.  A take plays as in the
// main game: the move travels down from its place in the graph into the player's tray and its tax into the taxman's,
// one after another.  A number that leaves the pot is gone from the graph, and so is
// every line that touches it; since a number leaves the pot only after the divisors it taxes, a line that goes leads
// only to numbers already gone, and a number left with no line is one that cannot be taken.  Taking back, or
// starting over, brings the numbers back to their places with their lines.  Pointing at or focusing a number that can
// be taken puts a halo on its tax, everything still in the pot that it reaches by its lines, and on the lines it
// follows.  On a touch screen the first tap shows the tax and the second takes the number.
//
// URL parameter: graph=N or graph=N:m1,m2,... opens game N with those moves played.

import * as M from '../js/model.js';
import { mountPlay } from '../js/play.js';
import { scene } from '../js/scene.js';
import { boxAround, glide, makePiece, reserveRows, returnDelays, startOf, traysHTML } from '../js/trays.js';
import { MOTION, NS, SHAPE, SIZE, listWords, look, reducedMotion, restyle } from '../js/vocab.js';

export function mountGraphGame(host, { small, n = 21 }) {
  let view;
  const game = mountPlay(host, { small, n, param: 'graph', board: (context) => (view = graphBoard(context)) });
  return { ...game, highlight: (x, on) => view.highlight(x, on) };
}

const ROW = 38; // the distance between rows, in the units of a token, which is 30 across as a circle
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
  let slots = {}; // the two trays: player, taxman
  let held = new Map(); // the piece in a tray of each number taken
  let batch = 0; // the id of the latest take, which the pieces of one take share
  let pending = new Map(); // the timers of what happens when a piece sets out or lands: id -> what it does
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
      const base = { ...lay.at.get(x), scale: lay.scale, shape: SHAPE.plain };
      if (state.pot.has(x)) {
        v[x] = { ...base, ...(legal.has(x) ? {} : look('lying')), halo: halo.has(x) && x !== pointed ? 1 : 0 };
      } else v[x] = base; // gone from the graph: the board hides it, so it stays where it stands
    }
    for (const [id, x, f] of lines) v[id] = { bend: 0, sag: 0, halo: halo.has(x) && halo.has(f) ? 1 : 0 };
    // The lines of a tax lie over the others.
    for (const k of [0, 1]) {
      for (const [id, x, f] of lines) {
        if ((halo.has(x) && halo.has(f) ? 1 : 0) === k) s.layer.line.appendChild(s.node(id));
      }
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
    flush();
    n = newN;
    lay = layout(n, width, deepest);
    pointed = null;
    shown = '';
    held = new Map();
    slots.player.replaceChildren();
    slots.taxman.replaceChildren();
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

  // Do fn after ms; flush() does at once whatever still waits.
  function later(fn, ms) {
    const id = setTimeout(() => {
      pending.delete(id);
      fn();
    }, ms);
    pending.set(id, fn);
  }

  function flush() {
    const todo = [...pending];
    pending = new Map();
    for (const [id, fn] of todo) {
      clearTimeout(id);
      fn();
    }
  }

  // A number or a line leaves the graph or comes back, at ms.
  function hide(node, gone, ms) {
    if (node.classList.contains('is-gone') === gone) return;
    if (ms > 0) later(() => node.classList.toggle('is-gone', gone), ms);
    else node.classList.toggle('is-gone', gone);
  }

  // Where on the screen the token of x stands.
  function centerOf(x) {
    const r = s.svg.getBoundingClientRect();
    return { x: r.left + lay.at.get(x).x, y: r.top + lay.at.get(x).y };
  }

  const lookNames = (tray) => (tray === 'player' ? ['plain', 'player'] : ['plain', 'taxman']);

  // Put the pieces of the trays where the state says.  A piece of a number taken in this step (order) travels from its
  // token, and one that the state no longer holds travels back to it.  Returns when each number leaves the graph or
  // returns to it, in milliseconds from now.
  function trays(order, wasLegal, animate) {
    const at = new Map();
    const target = new Map();
    for (const x of state.player) target.set(x, 'player');
    for (const x of state.taxman) target.set(x, 'taxman');
    // The pieces the state does not hold go back to their tokens, the last to leave the first to return.
    const back = [...held].filter(([x]) => !target.has(x));
    const delays = returnDelays(back.map(([, p]) => ({ step: p._batch, place: p._k })));
    for (const [i, [x, p]] of back.entries()) {
      held.delete(x);
      const delay = delays[i];
      at.set(x, animate ? delay + MOTION.move : 0);
      if (!animate) {
        p.remove();
        continue;
      }
      const c = centerOf(x);
      const near = legal.has(x) ? ['plain'] : ['plain', 'lying'];
      const box = boxAround(p, c.x, c.y, lay.scale);
      glide(p, { to: box, delay, looks: near });
      later(() => p.remove(), delay + MOTION.move);
    }
    // The pieces of the state, in the order of its lists, so that the trays fill left to right.
    const fresh = [];
    for (const [x, tray] of target) {
      let p = held.get(x);
      if (!p) {
        const coming = animate && order.includes(x);
        const near = wasLegal.has(x) ? ['plain'] : ['plain', 'lying'];
        p = makePiece(x, ...(coming ? near : lookNames(tray)));
        p.disabled = true;
        p._tray = tray;
        p._batch = batch;
        p._k = Math.max(0, order.indexOf(x));
        held.set(x, p);
        if (coming) fresh.push(x);
      }
      p._final = lookNames(tray);
      p.setAttribute('aria-label', label(x));
    }
    for (const tray of ['player', 'taxman']) {
      const list = tray === 'player' ? state.player : state.taxman;
      list.forEach((x, i) => {
        const p = held.get(x);
        if (slots[tray].children[i] !== p) slots[tray].insertBefore(p, slots[tray].children[i] ?? null);
      });
    }
    // Each sets out as its token leaves the graph.
    for (const x of fresh) {
      const p = held.get(x);
      const c = centerOf(x);
      const delay = startOf(p._k);
      at.set(x, delay);
      const from = boxAround(p, c.x, c.y, lay.scale);
      glide(p, { from, delay, looks: p._final });
    }
    return at;
  }

  // Take the numbers that left the pot, and the lines that touch them, out of the graph, and bring back the ones that
  // returned: a line goes with the first of its two numbers to go, and comes back with the last to come back.
  function gone(at) {
    const was = new Map();
    for (let x = 1; x <= n; x++) was.set(x, s.node(x).classList.contains('is-gone'));
    const ms = (x) => at.get(x) ?? 0;
    for (let x = 1; x <= n; x++) hide(s.node(x), !state.pot.has(x), ms(x));
    for (const [id, x, f] of lines) {
      const ends = [x, f].filter((e) => was.get(e) !== !state.pot.has(e));
      const out = !state.pot.has(x) || !state.pot.has(f);
      const when = out ? Math.min(...ends.map(ms)) : Math.max(...ends.map(ms));
      hide(s.node(id), out, ends.length ? when : 0);
    }
  }

  // Draw the state.  A move lands as in the main game: the move first, then its tax and the rest, one after another.
  function show(next, { legal: open, order = [] }) {
    flush();
    const wasLegal = legal;
    state = next;
    legal = open;
    attributes();
    host.querySelector('.pot-sum').textContent = state.pot.size ? `${state.pot.size} left` : 'empty';
    const key = `${[...state.pot]}|${state.player}|${state.taxman}`;
    if (key === shown && order.length === 0) return;
    shown = key;
    if (order.length) batch++;
    gone(trays(order, wasLegal, !reducedMotion()));
    // A number taken by the keyboard leaves the graph: the focus goes to the first number that can be taken now.
    const focused = Number(document.activeElement?.closest?.('[data-n]')?.dataset.n);
    if (s.svg.contains(document.activeElement) && !state.pot.has(focused)) {
      const next = lay.order.find((x) => legal.has(x));
      if (next !== undefined) s.node(next).focus({ preventScroll: true });
    }
    if (order.length === 0) {
      draw(0);
      return;
    }
    // A number left with no line lies down as the last lands.
    const last = startOf(order.length - 1);
    const windows = {};
    for (let x = 1; x <= n; x++) if (state.pot.has(x)) windows[x] = [last / (last + MOTION.move), 1];
    draw(last + MOTION.move, windows);
  }

  function refuse(x) {
    const node = s.node(x);
    node.classList.remove('is-refused');
    void node.getBoundingClientRect();
    node.classList.add('is-refused');
  }

  // A new width lays the board out again; its height is fixed for n and the width.
  function reserve() {
    reserveRows(host, slots.taxman, n);
    const width = wrap.getBoundingClientRect().width;
    if (Math.abs(width - lay.width) < 0.5) return;
    lay = layout(n, width, deepest);
    size();
    draw(0);
  }

  return {
    html: `
    <div class="tray-box pot-box graph-board">
      <header><span>The pot</span><span class="pot-sum"></span></header>
      <div class="graph-wrap"></div>
    </div>
    ${traysHTML(you)}`,
    mount(h) {
      host = h;
      wrap = host.querySelector('.graph-wrap');
      slots = { player: host.querySelector('.player .slots'), taxman: host.querySelector('.taxman .slots') };
    },
    build,
    show,
    refuse,
    reserve,
    settle() {
      flush();
      for (const p of held.values()) {
        for (const a of p.getAnimations({ subtree: true })) a.finish();
        restyle(p.querySelector('svg'), ...p._final);
      }
      s.stop();
      draw(0);
    },
    highlight: (x, on) => s?.highlight(x, on),
  };
}
