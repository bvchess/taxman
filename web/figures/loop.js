// The loop: 10, 14, 15 and 21, each the product of two primes, make a square on 2, 5, 3 and 7.  The reader plays
// them, with only the four primes as tax, by the rules of js/model.js, and finds that one is always stuck.  A product
// is a circle, a raised coin the reader presses, while one of its primes is still in the pot; a product with none left
// is stuck, and its circle turns into a square, flat.  The primes are squares.  A number taken lies flat in a tray: the
// player's a blue circle, the taxman's an orange square (js/trays.js has the looks).  The claim behind the figure (the
// four are not playable, any three are) is checked with the model's isPlayable when the figure is made.
//
// The figure has two forms of the same graph, and the switch of figures/layer-view.js ("Show: in layers / as a loop")
// moves between them.  In layers it is the maximal-factor graph of the game above it: the four products in a column
// at the left, the four primes in a column at the right, level with them, each product joined by a line to its two
// primes; the loop is there, but the lines cross.  In the loop each product stands between its two primes, on a side
// of the square its lines make.  The switch moves every number still on the table to its place in the other form in
// one motion, each along a smooth curve that passes through its place in a midway pose, chosen so that no two numbers
// touch on the way, either way (scene.js, through).  The reader may play in either form and switch at any time; the
// trays and Start over do not depend on the form.  The "stuck" label stands by the stuck number in the loop, and in
// layers in the primes' column, which the game has emptied, level with the stuck numbers.
//
// URL parameter: loop=10,14 opens the figure with those moves played, in layers; loop=unfolded:10,14, or just
// loop=unfolded, opens it as a loop.

import * as M from '../js/model.js';
import { TRAY_LOOK, potLook } from '../js/trays.js';
import { el, listWords, look } from '../js/vocab.js';
import { scene } from '../js/scene.js';
import { viewSwitch } from './layer-view.js';

const W = 360;
const H = 372;
// The square: primes at the corners, each product on the side between its two primes.
const CORNER = { 2: [70, 234], 5: [290, 234], 3: [290, 54], 7: [70, 54] };
const PRODUCTS = [10, 15, 21, 14];
// Layers: the products ascending up a column at the left, the primes ascending up a column at the right, level.
const COLUMN = { product: 90, prime: 270 };
const rowY = (i) => 234 - 60 * i;
// The places the numbers pass through between the two forms, where they pass one another.  Each number moves along the
// smooth curve through its place; the places were found by moving them a little at a time, keeping any move that kept
// the numbers farther apart, until, unfolding or folding, no two centers came nearer than 55 units.
const MIDWAY = {
  2: [158, 253], 3: [267, 134], 5: [323, 128], 7: [155, 29],
  10: [153, 197], 14: [52, 152], 15: [193, 141], 21: [116, 95],
};
const TRAY = { y: 280, h: 86, w: 168, x: { player: 8, taxman: 184 } };
const SCALE = 1.25;
const CROSSING = 1500; // the switch of form, in milliseconds (two beats of 900 took 1800)

export function mountLoop(host) {
  const primes = Object.keys(CORNER).map(Number);
  for (const m of PRODUCTS) {
    const f = M.maximalFactors(m);
    if (f.length !== 2 || !f.every((p) => CORNER[p])) throw new Error(`the loop: ${m} is not a side of the square`);
  }
  if (M.isPlayable(PRODUCTS)) throw new Error('the loop: the four numbers should not be playable');
  for (const m of PRODUCTS) {
    if (!M.isPlayable(PRODUCTS.filter((x) => x !== m))) throw new Error('the loop: every three should be playable');
  }

  const byValue = (xs) => [...xs].sort((a, b) => a - b);
  const mid = (m) => {
    const [a, b] = M.maximalFactors(m).map((p) => CORNER[p]);
    return [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
  };
  // Every number's place in each pose.
  const PLACE = {
    loop: { ...CORNER, ...Object.fromEntries(PRODUCTS.map((m) => [m, mid(m)])) },
    layers: {
      ...Object.fromEntries(primes.map((p, i) => [p, [COLUMN.prime, rowY(i)]])),
      ...Object.fromEntries(byValue(PRODUCTS).map((m, i) => [m, [COLUMN.product, rowY(i)]])),
    },
  };

  const s = scene(host.querySelector('svg'), { width: W, height: H, rendering: 'raised' });
  for (const who of ['player', 'taxman']) {
    const g = el('g', {});
    el('rect', { x: TRAY.x[who], y: TRAY.y, width: TRAY.w, height: TRAY.h, rx: 16, class: `tray tray-${who}`, 'stroke-width': 1.5 }, g);
    const t = el('text', { x: TRAY.x[who] + 14, y: TRAY.y + 23, class: `v-text v-text-name tray-name-${who}` }, g);
    t.textContent = who === 'player' ? 'You' : 'Taxman';
    s.custom(`tray-${who}`, g, () => {});
  }
  for (const x of [...primes, ...PRODUCTS]) s.number(x, x);
  const strings = [];
  for (const m of PRODUCTS) for (const p of M.maximalFactors(m)) {
    s.line(`${m}-${p}`, m, p, 'thin');
    strings.push([m, p]);
  }
  s.text('stuck', (pos, look) => look.words ?? '', { anchor: 'middle' });
  const traySlot = (who, i) => ({ x: TRAY.x[who] + 28 + 37 * i, y: TRAY.y + 56, scale: 0.92 });

  const status = host.querySelector('.loop-status');
  let state;
  let form = 'layers'; // 'layers' or 'loop': the form the figure is in, or moving to

  const pass = (x, crossing) => (crossing ? { through: { x: MIDWAY[x][0], y: MIDWAY[x][1] } } : {});

  // The looks of the numbers on the table, in the form's pose.  crossing is a switch of form, in which
  // the numbers on the table pass through their midway places.
  function looks(crossing) {
    const legal = new Set(M.legalMoves(state));
    const at = PLACE[form];
    const v = { 'tray-player': {}, 'tray-taxman': {} };
    for (const p of primes) {
      const i = state.taxman.indexOf(p);
      v[p] = i < 0
        ? { ...look(...potLook(false)), x: at[p][0], y: at[p][1], scale: SCALE, ...pass(p, crossing) }
        : { ...look(...TRAY_LOOK.taxman), ...traySlot('taxman', i) };
    }
    const stuck = [];
    for (const m of PRODUCTS) {
      const i = state.player.indexOf(m);
      const [x, y] = at[m];
      const way = pass(m, crossing);
      if (i >= 0) v[m] = { ...look(...TRAY_LOOK.player), ...traySlot('player', i) };
      else {
        v[m] = { ...look(...potLook(legal.has(m))), x, y, scale: SCALE, ...way };
        if (!legal.has(m)) stuck.push(m);
      }
    }
    stuck.sort((a, b) => a - b);
    // The lines run straight in every pose, as in the maximal-factor graph above.
    for (const [m, p] of strings) if (state.pot.has(m) && state.pot.has(p)) v[`${m}-${p}`] = { sag: 0 };
    if (stuck.length) {
      let [sx, sy] = [W / 2, 150];
      if (form === 'layers') {
        // Every prime is gone by now: the label stands in the primes' column, level with the stuck numbers.
        const ys = stuck.map((m) => PLACE.layers[m][1]);
        [sx, sy] = [COLUMN.prime, (Math.min(...ys) + Math.max(...ys)) / 2 + 4];
      } else if (stuck.length === 1) {
        // A lone stuck number is labeled just above it: inside the square, or over it for the top side, never down by
        // the trays.
        const [x, y] = PLACE.loop[stuck[0]];
        [sx, sy] = [x, y - 32];
      }
      const gone = listWords(M.maximalFactors(stuck[0]).sort((a, b) => a - b));
      const words = stuck.length === 1 ? `stuck: ${gone} are gone` : `${listWords(stuck)} are stuck`;
      v.stuck = { x: sx, y: sy, words };
    }
    return { v, stuck, over: legal.size === 0 };
  }

  function say(stuck, over, last) {
    if (!last) {
      status.innerHTML = 'Take one of the four numbers. The taxman takes its primes still on the table.';
      return;
    }
    const took = `You took ${last.move}; the taxman took ${listWords(last.tax)}.`;
    if (!over) {
      status.innerHTML = `${took} ${state.player.length === 1 ? 'The first move takes both of its primes.' : ''}`;
      return;
    }
    const words = ['none', 'one', 'two', 'three', 'four'];
    status.innerHTML = `${took} <strong>${listWords(stuck)} ${stuck.length > 1 ? 'are' : 'is'} stuck</strong>: ${
      stuck.length > 1 ? 'their primes are gone' : `both of ${stuck[0]}’s primes are gone`
    }. You played ${words[state.player.length]} of the four. Four numbers need four primes to pay with, and the first move took two of them.`;
  }

  // Show the state in the form's pose.  staged is a take, whose move and tax go one after another; crossing is a switch
  // of form.
  function render(animate, { staged = false, crossing = false } = {}) {
    const { v, stuck, over } = looks(crossing);
    for (const m of PRODUCTS) {
      const node = s.node(m);
      const canTake = state.pot.has(m) && M.legalMoves(state).includes(m);
      node.setAttribute('tabindex', canTake ? '0' : '-1');
      node.setAttribute('role', 'button');
      node.setAttribute('aria-disabled', String(!canTake));
      node.setAttribute('aria-label', state.pot.has(m) ? `${m} = ${M.maximalFactors(m).sort((a, b) => a - b).join(' × ')}${canTake ? '' : ', stuck'}` : `${m}, taken`);
      node.classList.toggle('is-takeable', canTake);
    }
    const last = state.log[state.log.length - 1];
    say(stuck, over, last);
    if (last && staged) {
      v[last.move].at = [0, 0.65];
      last.tax.forEach((t, j) => (v[t].at = [0.25 + 0.12 * j, 0.85 + 0.12 * j]));
    }
    return s.morph(v, { duration: animate ? (crossing ? CROSSING : 900) : 0 });
  }

  function take(m) {
    if (!state.pot.has(m) || !M.legalMoves(state).includes(m)) return false;
    state = M.play(state, m);
    render(true, { staged: true });
    return true;
  }

  function reset() {
    state = { n: 21, pot: new Set([...primes, ...PRODUCTS]), player: [], taxman: [], log: [] };
    render(false);
  }

  // Move to the other form in one motion.
  const switcher = viewSwitch({ layers: 'in layers', loop: 'as a loop' }, (to) => {
    if (to === form) return;
    form = to;
    switcher.show(form);
    render(true, { crossing: true });
  });
  host.insertBefore(switcher.box, host.querySelector('svg'));

  for (const m of PRODUCTS) {
    const node = s.node(m);
    node.addEventListener('click', () => take(m));
    node.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        take(m);
      }
    });
  }
  host.querySelector('[data-act="reset"]').addEventListener('click', reset);
  const asked = (new URLSearchParams(location.search).get('loop') ?? '').split(/[,:]/);
  if (asked.includes('unfolded')) form = 'loop';
  switcher.show(form);
  reset();
  for (const m of asked.map(Number)) if (state.pot.has(m) && M.legalMoves(state).includes(m)) state = M.play(state, m);
  render(false);
  return { highlight: (n, on) => s.highlight(n, on) };
}
