// The loop: 10, 14, 15 and 21, each the product of two primes, make a square on 2, 5, 3 and 7.  The reader plays
// them, with only the four primes as tax, by the rules of js/model.js, and finds that one is always stuck.  The
// numbers are raised tokens, since the reader takes them.  The claim behind the figure (the four are not playable,
// any three are) is checked with the model's isPlayable when the figure is made.
//
// URL parameter: loop=10,14 opens the figure with those moves played.

import * as M from '../js/model.js';
import { el, listWords } from '../js/vocab.js';
import { scene } from '../js/scene.js';

const W = 360;
const H = 372;
// The square: primes at the corners, each product on the side between its two primes.
const CORNER = { 2: [70, 54], 5: [290, 54], 3: [290, 234], 7: [70, 234] };
const PRODUCTS = [10, 15, 21, 14];
const TRAY = { y: 280, h: 86, w: 168, x: { player: 8, taxman: 184 } };
const SCALE = 1.25;

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

  const s = scene(host.querySelector('svg'), { width: W, height: H, rendering: 'raised' });
  const [x0, y0] = CORNER[2];
  const [x1, y1] = CORNER[3];
  s.custom('square', el('rect', {
    x: x0, y: y0, width: x1 - x0, height: y1 - y0, rx: 6, fill: 'none', 'stroke-width': 2,
    'stroke-dasharray': '3 6', 'stroke-linecap': 'round', style: 'stroke: var(--hair)',
  }), () => {});
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
  const mid = (m) => {
    const [a, b] = M.maximalFactors(m).map((p) => CORNER[p]);
    return [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
  };
  const traySlot = (who, i) => ({ x: TRAY.x[who] + 28 + 37 * i, y: TRAY.y + 56, scale: 0.92 });

  const status = host.querySelector('.loop-status');
  let state;

  function looks() {
    const legal = new Set(M.legalMoves(state));
    const over = legal.size === 0;
    const v = { square: {}, 'tray-player': {}, 'tray-taxman': {} };
    for (const p of primes) {
      const i = state.taxman.indexOf(p);
      v[p] = i < 0
        ? { shape: 0, x: CORNER[p][0], y: CORNER[p][1], scale: SCALE }
        : { shape: 0, taxman: 1, ...traySlot('taxman', i) };
    }
    const stuck = [];
    for (const m of PRODUCTS) {
      const i = state.player.indexOf(m);
      const [x, y] = mid(m);
      if (i >= 0) v[m] = { shape: 1, player: 1, ...traySlot('player', i) };
      else if (over) {
        v[m] = { shape: 1, dashed: 1, depth: 0, faded: 0.5, x, y, scale: SCALE };
        stuck.push(m);
      } else v[m] = { shape: 1, x, y, scale: SCALE, ...(legal.has(m) ? {} : { depth: 0, faded: 0.55 }) };
    }
    stuck.sort((a, b) => a - b);
    for (const [m, p] of strings) if (state.pot.has(m) && state.pot.has(p)) v[`${m}-${p}`] = {};
    if (stuck.length === 1) {
      const [x, y] = mid(stuck[0]);
      v.stuck = { x, y: y + (y > 150 ? 38 : -32), words: `stuck: ${listWords(M.maximalFactors(stuck[0]).sort((a, b) => a - b))} are gone` };
    } else if (stuck.length > 1) v.stuck = { x: W / 2, y: 150, words: `${listWords(stuck)} are stuck` };
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

  function render(animate) {
    const { v, stuck, over } = looks();
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
    if (last) {
      v[last.move].at = [0, 0.65];
      last.tax.forEach((t, j) => (v[t].at = [0.25 + 0.12 * j, 0.85 + 0.12 * j]));
    }
    return s.morph(v, { duration: animate ? 900 : 0 });
  }

  function take(m) {
    if (!state.pot.has(m) || !M.legalMoves(state).includes(m)) return false;
    state = M.play(state, m);
    render(true);
    return true;
  }

  function reset() {
    state = { n: 21, pot: new Set([...primes, ...PRODUCTS]), player: [], taxman: [], log: [] };
    render(false);
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
  host.querySelector('[data-act="reset"]').addEventListener('click', reset);
  reset();
  const asked = new URLSearchParams(location.search).get('loop');
  if (asked) {
    for (const m of asked.split(',').map(Number)) if (state.pot.has(m) && M.legalMoves(state).includes(m)) state = M.play(state, m);
    render(false);
  }
  return { highlight: (n, on) => s.highlight(n, on) };
}
