// Peel, step by step, on game 21's optimal set.  A flat figure: the set's nine numbers are gray circles in a column at
// the left (they are in the set of moves from the start), and the maximal factors outside the set that they list
// are squares in a column at the right, each number
// joined by a thin line to each factor it can pay with.  The time slider steps through one run of peel, one pair a
// step: the pair is drawn with the thick line of a payment, the factors outside the set stay unfilled squares, and a
// label names the kind of step, "one way to pay" for a front move (beside the number) or "one
// taker" for a back move (beside the factor).  The line grows from the end that acts: from the number to its factor
// for a front move, from the factor to its number for a back move.  At the end every number is paired, in the
// player's and the taxman's colors, as in the orders figure.
//
// The set and the game come from data/small.json, and the run is js/model.js's peel, choosing its steps: first the
// four the text walks through, then back moves, then the rest.  The pairs are checked against the payments of the
// recorded game, so a run that did not end in them would fail loudly.  Only the places are written by hand.
//
// URL parameter: peel=k opens state k (1, the set, to 11, all paired).

import * as M from '../js/model.js';
import { listWords } from '../js/vocab.js';
import { scene, timeline } from '../js/scene.js';

const N = 21;
const W = 440;
const H = 382;
const MEMBER_X = 165;
const FACTOR_X = 275;
const LABEL_GAP = 28;
const HEAD_Y = [16, 36]; // the two lines of the heads
const rowY = (i) => 72 + 36 * i;
// The numbers top to bottom, by hand, so that no two lines cross; each factor stands level with the number it pays
// for, so every payment is drawn straight across.
const MEMBER_ORDER = [16, 20, 12, 18, 14, 21, 9, 15, 19];
// The steps the text walks through first, each a front move: 19 with 1, 9 with 3, then 15 and 21.
const TEXT_RUN = [19, 9, 15, 21];
const KIND = { front: 'one way to pay', back: 'one taker' };

// A run of peel that opens as the text does, then takes the back moves that no front move could take, then the rest.
function runOf(set) {
  const paired = new Set();
  const { playable, steps } = M.peel(set, {
    choose(options) {
      const next = TEXT_RUN.find((m) => !paired.has(m));
      let pick = next === undefined ? null : options.find((o) => o.kind === 'front' && o.member === next);
      if (next !== undefined && !pick) throw new Error(`peel figure: ${next} is not a front move when the text takes it`);
      pick ??= options.find((o) => o.kind === 'back' && !options.some((p) => p.kind === 'front' && p.member === o.member));
      pick ??= options[0];
      paired.add(pick.member);
      return pick;
    },
  });
  if (!playable) throw new Error(`peel figure: game ${N}'s set should be playable`);
  return steps;
}

function facts(small) {
  const game = small.find((g) => g.n === N);
  const set = [...game.moves].sort((a, b) => a - b);
  const inSet = new Set(set);
  const lists = new Map(set.map((m) => [m, M.maximalFactors(m).filter((f) => !inSet.has(f))]));
  const pool = [...new Set([...lists.values()].flat())].sort((a, b) => a - b);
  const steps = runOf(set);
  const factorOf = new Map(steps.map((st) => [st.member, st.factor]));
  // The pairs are the payments of the recorded game: each move takes one tax, its factor.
  const payments = new Map(M.replay(N, game.moves).log.filter((e) => e.move !== undefined).map((e) => [e.move, e.tax]));
  for (const m of set) {
    const tax = payments.get(m);
    if (tax.length !== 1 || tax[0] !== factorOf.get(m)) throw new Error(`peel figure: ${m} pairs with ${factorOf.get(m)}, but pays ${tax}`);
  }
  const key = (xs) => [...xs].sort((a, b) => a - b).join(',');
  if (key(MEMBER_ORDER) !== key(set)) throw new Error('peel figure: the column of numbers is not the set');
  if (pool.some((f) => inSet.has(f)) || key(pool) !== key(steps.map((st) => st.factor))) {
    throw new Error('peel figure: the factors are not one each');
  }
  return { game, set, lists, pool, steps, factorOf };
}

// The words under the figure, for state k.
function words(f, k) {
  const { steps } = f;
  if (k === 0) {
    return `Game ${N}'s optimal set, its ${f.set.length} numbers at the left, and at the right the maximal factors outside it. A thin line joins a number to each factor it can pay with.`;
  }
  if (k > steps.length) {
    return `Peel pairs every number, so the set is playable. The pairs are the payments of the game itself: in each order above, every number, in the player's blue, takes its own factor, in the taxman's orange, as tax.`;
  }
  // Who is still in play just before the step.
  const done = steps.slice(0, k - 1);
  const goneM = new Set(done.map((st) => st.member));
  const goneF = new Set(done.map((st) => st.factor));
  const { kind, member: m, factor: x } = steps[k - 1];
  const many = (xs, one, more) => `${listWords(xs)} ${xs.length > 1 ? more : one}`;
  if (kind === 'front') {
    const losing = f.set.filter((y) => y !== m && !goneM.has(y) && f.lists.get(y).includes(x));
    const then = losing.length ? ` With ${x} gone, ${many(losing, 'has', 'have')} one way fewer to pay.` : '';
    return `One way to pay: ${m} has only ${x} left to pay with, so peel pairs them.${then}`;
  }
  const losing = f.lists.get(m).filter((g) => g !== x && !goneF.has(g));
  const then = losing.length ? ` With ${m} gone, ${many(losing, 'has', 'have')} one taker fewer.` : '';
  return `One taker: only ${m} can still use ${x}, so peel pairs them.${then}`;
}

/** Mount the figure in host, which holds an <svg>, .fig-timeline and .fig-caption. */
export function mountPeel(host, { small }) {
  const f = facts(small);
  const { steps } = f;
  const s = scene(host.querySelector('svg'), {
    width: W, height: H, className: 'is-dense',
    label: `Peel, step by step, on game ${N}'s optimal set. The text under the figure describes the current step.`,
  });
  const row = new Map(MEMBER_ORDER.map((m, i) => [m, rowY(i)]));
  for (const m of MEMBER_ORDER) row.set(f.factorOf.get(m), row.get(m));
  const edges = f.set.flatMap((m) => f.lists.get(m).map((x) => [m, x]));
  for (const m of f.set) s.number(m, m);
  for (const x of f.pool) s.number(x, x);
  for (const [m, x] of edges) s.line(`e${m}-${x}`, m, x, 'thin');
  for (const st of steps) {
    const [from, to] = st.kind === 'front' ? [st.member, st.factor] : [st.factor, st.member];
    s.line(`p${st.member}`, from, to, 'pay');
  }
  steps.forEach((st, i) => s.text(`kind${i}`, KIND[st.kind], { anchor: st.kind === 'front' ? 'end' : 'start' }));
  // The heads face each other over the two columns, the factors' in two lines, so that it fits a phone.
  s.text('headSet', 'the set', { kind: 'name', anchor: 'end' });
  s.text('headFactors', 'maximal factors', { kind: 'name', anchor: 'start' });
  s.text('headFactors2', 'outside the set', { kind: 'name', anchor: 'start' });

  const last = steps.length + 1;
  const states = [];
  for (let k = 0; k <= last; k++) {
    const doneM = new Set(steps.slice(0, k).map((st) => st.member));
    const doneF = new Set(steps.slice(0, k).map((st) => st.factor));
    const newest = k >= 1 && k <= steps.length ? steps[k - 1] : null;
    const end = k === last;
    const v = {
      headSet: { x: MEMBER_X + 16, y: HEAD_Y[1] },
      headFactors: { x: FACTOR_X - 14, y: HEAD_Y[0] },
      headFactors2: { x: FACTOR_X - 14, y: HEAD_Y[1] },
    };
    // A member is a gray circle throughout; a factor stays an unfilled square; the pairing is the thick line.
    const mark = (x, shape, isNew, color) => ({
      x: shape ? MEMBER_X : FACTOR_X, y: row.get(x), shape, kept: shape,
      ...(end ? { [color]: 1 } : {}), ...(isNew ? { halo: 1, at: [0.1, 0.6] } : {}),
    });
    for (const m of f.set) v[m] = mark(m, 1, newest?.member === m, 'player');
    for (const x of f.pool) v[x] = mark(x, 0, newest?.factor === x, 'taxman');
    // A thin line lives while both its ends are in play; one the step takes out fades once the payment is drawn.
    const before = new Set(steps.slice(0, Math.max(0, k - 1)).flatMap((st) => [st.member, st.factor]));
    for (const [m, x] of edges) {
      const id = `e${m}-${x}`;
      if (!doneM.has(m) && !doneF.has(x)) v[id] = {};
      else if (newest && !before.has(m) && !before.has(x)) v[id] = { opacity: 0, at: [0.35, 0.85] };
    }
    steps.forEach((st, i) => {
      if (i < k) v[`p${st.member}`] = i === k - 1 && newest ? { at: [0.05, 0.55] } : {};
      else if (i === k) v[`p${st.member}`] = { draw: 0 }; // grows from the acting end in the next step
    });
    if (newest) {
      const i = k - 1;
      const front = newest.kind === 'front';
      v[`kind${i}`] = { x: front ? MEMBER_X - LABEL_GAP : FACTOR_X + LABEL_GAP, y: row.get(newest.member), halo: 1, at: [0.3, 0.8] };
    }
    states.push(v);
  }
  s.states(states);

  const names = ['the set', ...steps.map((st) => String(st.member)), 'all paired'];
  const caption = host.querySelector('.fig-caption');
  timeline(s, host.querySelector('.fig-timeline'), {
    stops: names.map((name, at) => ({ at, name })),
    param: 'peel',
    fromParam: (value) => Number(value) - 1,
    toParam: (k) => String(k + 1),
    describe: (k) => words(f, k),
    title: (k) => {
      caption.textContent = words(f, k);
      host.dataset.state = String(k + 1);
    },
    label: 'Peel figure: step',
  });
  return {
    goState: (k) => s.go(Number(k) - 1),
    stopName: (k) => names[Number(k) - 1],
    highlight: (n, on) => s.highlight(n, on),
  };
}
