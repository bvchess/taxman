// The loop: 10, 14, 15 and 21, each the product of two primes, make a square on 2, 5, 3 and 7.  The reader plays
// them, with only the four primes as tax, by the rules of js/model.js, and finds that one is always stuck.  The
// numbers are raised tokens, since the reader takes them.  The claim behind the figure (the four are not playable,
// any three are) is checked with the model's isPlayable when the figure is made.
//
// The figure has two forms of the same graph, and a button that unfolds one into the other.  In layers it is the
// maximal-factor graph of the game above it: the four products in a column at the left, the four primes in a column at
// the right, level with them, each product joined by a line to its two primes; the loop is there, but the lines
// cross.  In the loop each product stands between its two primes, on a side of the dashed square.  The button moves
// every number still on the table to its place in the other form in one motion, each along a smooth curve that passes
// through its place in a midway pose, chosen so that no two numbers touch on the way, either way (scene.js, through);
// and the square fades in as the unfolding ends and out as the folding starts.  The reader
// may play in either form and switch at any time; the trays and Start over do not depend on the form.  The "stuck"
// label stands by the stuck number in the loop, and in layers in the primes' column, which the game has emptied, level
// with the stuck numbers.
//
// URL parameter: loop=10,14 opens the figure with those moves played, in layers; loop=unfolded:10,14, or just
// loop=unfolded, opens it as a loop.

import * as M from '../js/model.js';
import { el, listWords } from '../js/vocab.js';
import { scene } from '../js/scene.js';

const W = 360;
const H = 372;
// The square: primes at the corners, each product on the side between its two primes.
const CORNER = { 2: [70, 54], 5: [290, 54], 3: [290, 234], 7: [70, 234] };
const PRODUCTS = [10, 15, 21, 14];
// Layers: the products ascending down a column at the left, the primes ascending down a column at the right, level.
const COLUMN = { product: 90, prime: 270 };
const rowY = (i) => 54 + 60 * i;
// The places the numbers pass through between the two forms, where they pass one another.  Each number moves along the
// smooth curve through its place; the places were found by moving them a little at a time, keeping any move that kept
// the numbers farther apart, until, unfolding or folding, no two centers came nearer than 55 units.
const MIDWAY = {
  2: [158, 35], 3: [267, 154], 5: [323, 160], 7: [155, 259],
  10: [153, 91], 14: [52, 136], 15: [193, 147], 21: [116, 193],
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
  const [x0, y0] = CORNER[2];
  const [x1, y1] = CORNER[3];
  s.custom('square', el('rect', {
    x: x0, y: y0, width: x1 - x0, height: y1 - y0, rx: 6, fill: 'none', 'stroke-width': 2,
    'stroke-dasharray': '3 6', 'stroke-linecap': 'round', style: 'stroke: var(--hair)',
  }), (node, look) => { node.style.opacity = look.opacity; });
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
  const toggle = host.querySelector('[data-act="unfold"]');
  let state;
  let form = 'layers'; // 'layers' or 'loop': the form the figure is in, or moving to

  const pass = (x, crossing) => (crossing ? { through: { x: MIDWAY[x][0], y: MIDWAY[x][1] } } : {});

  // The looks of the numbers on the table and the square, in the form's pose.  crossing is a switch of form, in which
  // the numbers on the table pass through their midway places.
  function looks(crossing) {
    const legal = new Set(M.legalMoves(state));
    const over = legal.size === 0;
    const at = PLACE[form];
    // The square fades in as a move into the loop ends and out as a move out of it starts.
    const into = form === 'loop';
    const square = { opacity: into ? 1 : 0, at: into ? [0.45, 1] : [0, 0.55] };
    const v = { square, 'tray-player': {}, 'tray-taxman': {} };
    for (const p of primes) {
      const i = state.taxman.indexOf(p);
      v[p] = i < 0
        ? { shape: 0, x: at[p][0], y: at[p][1], scale: SCALE, ...pass(p, crossing) }
        : { shape: 0, taxman: 1, ...traySlot('taxman', i) };
    }
    const stuck = [];
    for (const m of PRODUCTS) {
      const i = state.player.indexOf(m);
      const [x, y] = at[m];
      const way = pass(m, crossing);
      if (i >= 0) v[m] = { shape: 1, player: 1, ...traySlot('player', i) };
      else if (over) {
        v[m] = { shape: 1, dashed: 1, depth: 0, faded: 0.5, x, y, scale: SCALE, ...way };
        stuck.push(m);
      } else v[m] = { shape: 1, x, y, scale: SCALE, ...way, ...(legal.has(m) ? {} : { depth: 0, faded: 0.55 }) };
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
        const [x, y] = PLACE.loop[stuck[0]];
        [sx, sy] = [x, y + (y > 150 ? 38 : -32)];
      }
      const gone = listWords(M.maximalFactors(stuck[0]).sort((a, b) => a - b));
      const words = stuck.length === 1 ? `stuck: ${gone} are gone` : `${listWords(stuck)} are stuck`;
      v.stuck = { x: sx, y: sy, words };
    }
    return { v, stuck, over };
  }

  function say(stuck, over, last) {
    if (!last) {
      status.innerHTML = 'Take one of the four coins. The taxman takes its primes still on the table.';
      return;
    }
    const took = `You took ${last.move}; the taxman took ${listWords(last.tax)}.`;
    if (!over) {
      status.innerHTML = `${took} ${state.player.length === 1 ? 'The first move takes both of its primes.' : ''}`;
      return;
    }
    const words = ['None', 'One', 'Two', 'Three', 'Four'];
    status.innerHTML = `${took} <strong>${listWords(stuck)} ${stuck.length > 1 ? 'are' : 'is'} stuck</strong>: ${
      stuck.length > 1 ? 'their primes are gone' : `both of ${stuck[0]}’s primes are gone`
    }. ${words[state.player.length]} of the four were played. Four numbers need four primes to pay with, and the first move took two of them.`;
  }

  // Show the state in the form's pose.  staged is a take, whose move and tax go one after another; crossing is a switch
  // of form.
  function render(animate, { staged = false, crossing = false } = {}) {
    const { v, stuck, over } = looks(crossing);
    for (const m of PRODUCTS) {
      const node = s.node(m);
      const canTake = state.pot.has(m) && M.legalMoves(state).includes(m);
      node.setAttribute('tabindex', state.pot.has(m) ? '0' : '-1');
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

  // Name the form on the button: it says what pressing it does.
  function label() {
    toggle.textContent = form === 'layers' ? 'Unfold into a loop' : 'Back to layers';
  }

  // Move to the other form in one motion.
  function unfold() {
    form = form === 'layers' ? 'loop' : 'layers';
    label();
    return render(true, { crossing: true });
  }

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
  toggle.addEventListener('click', unfold);
  host.querySelector('[data-act="reset"]').addEventListener('click', reset);
  const asked = (new URLSearchParams(location.search).get('loop') ?? '').split(/[,:]/);
  if (asked.includes('unfolded')) form = 'loop';
  label();
  reset();
  for (const m of asked.map(Number)) if (state.pot.has(m) && M.legalMoves(state).includes(m)) state = M.play(state, m);
  render(false);
  return { highlight: (n, on) => s.highlight(n, on) };
}
