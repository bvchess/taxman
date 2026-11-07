// The ladder of the counting theorem (C2, docs/theory/franklin-moniot.md), on three small sets.  A payment scheme
// gives each number of a set a maximal factor of its own, from outside the set, to pay with.  Count the schemes that
// pay with the same factors: a set has a scheme when some choice of factors gets at least one, passes the parity
// test (independent over GF(2) with its own coordinates) when some choice gets an odd number, and is playable when
// some choice gets exactly one.  The three sets climb the ladder a rung each:
//   6, 10 and 15, the triangle: two schemes, on the first rung only;
//   12, 18, 20, 30 and 45, the first odd core (C3): three schemes, on the first two;
//   9, 10 and 15, the triangle with 6 swapped for 9: one scheme, on all three.
// A flat figure: each scheme is drawn as the set's numbers (circles) and the factors outside it (squares), a thin line
// to each factor a number could pay with and a thick one to the factor it does.  Every count and every answer is
// computed here from js/model.js, and checked against the theorem: the odd count against the GF(2) test below, and
// exactly one against peel.  Only the places are written by hand.

import * as M from '../js/model.js';
import { listWords } from '../js/vocab.js';
import { scene } from '../js/scene.js';

// The places of each set's numbers and of the factors outside it, in figure units.  A number with two factors sits
// between them, so the triangle's numbers are its sides and its factors its corners.
const TRIANGLE = { 2: [90, 18], 3: [18, 142], 5: [162, 142] };
export const SETS = [
  { set: [6, 10, 15], w: 180, h: 160, at: { ...TRIANGLE, 6: [54, 80], 10: [126, 80], 15: [90, 142] } },
  {
    set: [12, 18, 20, 30, 45], w: 250, h: 232,
    at: { 6: [125, 18], 9: [232, 86], 15: [192, 214], 10: [58, 214], 4: [18, 86], 12: [71.5, 52], 18: [178.5, 52], 45: [212, 150], 20: [38, 150], 30: [125, 138] },
  },
  { set: [9, 10, 15], w: 180, h: 160, at: { ...TRIANGLE, 9: [54, 80], 10: [126, 80], 15: [90, 142] } },
];

/** The rungs, from the loosest test to the game itself: who decides, and the count of schemes it asks for. */
export const RUNGS = [
  { key: 'some', name: 'Franklín and Moniot’s bound', rule: 'at least one scheme' },
  { key: 'odd', name: 'The parity bound', rule: 'an odd number of schemes' },
  { key: 'one', name: 'Playable', rule: 'exactly one scheme' },
];

/** The factors each member of a set can pay with: its maximal factors outside the set. */
export function listsOf(set) {
  const inSet = new Set(set);
  return new Map(set.map((x) => [x, M.maximalFactors(x).filter((f) => !inSet.has(f))]));
}

/** The parity test: no group of the set's members lists every factor outside the set an even number of times, that
 *  is, their lists are linearly independent over GF(2) (docs/theory/playability.md, "Independence"). */
export function passesParityTest(set) {
  const basis = new Map(); // the highest factor of each vector kept -> the vector, a BigInt with a bit a factor
  for (const list of listsOf(set).values()) {
    let v = list.reduce((acc, f) => acc | (1n << BigInt(f)), 0n);
    while (v !== 0n) {
      const top = v.toString(2).length - 1;
      if (!basis.has(top)) {
        basis.set(top, v);
        break;
      }
      v ^= basis.get(top);
    }
    if (v === 0n) return false;
  }
  return true;
}

/** Every payment scheme of a set: a Map from each member to the factor it pays with, no two the same. */
export function schemesOf(set) {
  const lists = listsOf(set);
  const out = [];
  const walk = (i, used, pay) => {
    if (i === set.length) {
      out.push(new Map(pay));
      return;
    }
    for (const f of lists.get(set[i])) {
      if (used.has(f)) continue;
      used.add(f);
      pay.push([set[i], f]);
      walk(i + 1, used, pay);
      pay.pop();
      used.delete(f);
    }
  };
  walk(0, new Set(), []);
  return out;
}

/** A set's place on the ladder: its schemes, their count onto each set of factors, and the three answers. */
export function rungsOf(set) {
  const schemes = schemesOf(set);
  const onto = new Map();
  for (const s of schemes) {
    const key = [...s.values()].sort((a, b) => a - b).join(',');
    onto.set(key, (onto.get(key) ?? 0) + 1);
  }
  const counts = [...onto.values()];
  const answers = { some: counts.some((c) => c >= 1), odd: counts.some((c) => c % 2 === 1), one: counts.some((c) => c === 1) };
  // The counting theorem, checked: the odd count is the parity test, and exactly one is playability.
  if (answers.odd !== passesParityTest(set) || answers.one !== M.isPlayable(set)) {
    throw new Error(`ladder: the counts of ${set} disagree with the counting theorem`);
  }
  return { schemes, onto, answers };
}

const sorted = (xs) => [...xs].sort((a, b) => a - b);

// One scheme, drawn: the set's numbers, the factors outside it, a thin line to each listed factor and a thick one to
// the factor it pays with.
function drawScheme(host, spec, scheme, label) {
  const s = scene(host, { width: spec.w, height: spec.h, label, className: 'ladder-scheme' });
  const lists = listsOf(spec.set);
  const pool = sorted(new Set([...lists.values()].flat()));
  for (const x of [...spec.set, ...pool]) {
    if (!spec.at[x]) throw new Error(`ladder: no place for ${x}`);
    s.number(x, x);
  }
  const look = {};
  for (const x of spec.set) look[x] = { x: spec.at[x][0], y: spec.at[x][1], shape: 1 };
  for (const f of pool) look[f] = { x: spec.at[f][0], y: spec.at[f][1], shape: 0 };
  for (const [x, list] of lists) {
    for (const f of list) {
      const id = `${x}-${f}`;
      s.line(id, x, f, scheme.get(x) === f ? 'pay' : 'thin');
      look[id] = {};
    }
  }
  s.states([look]).show(0);
  return s;
}

/** Mount the figure in host, a figure holding .ladder and .fig-caption. */
export function mountLadder(host) {
  const grid = host.querySelector('.ladder');
  const head = document.createElement('div');
  head.className = 'ladder-head';
  head.innerHTML = `<span class="ladder-corner"></span>${RUNGS.map((r) => `<span class="rung-head"><b>${r.name}</b><span>${r.rule}</span></span>`).join('')}`;
  grid.append(head);
  const marks = [];
  for (const spec of SETS) {
    const { schemes, onto, answers } = rungsOf(spec.set);
    if (onto.size !== 1) throw new Error(`ladder: the schemes of ${spec.set} should all pay with the same factors`);
    const factors = [...onto.keys()][0].split(',').map(Number);
    const count = schemes.length;
    const row = document.createElement('div');
    row.className = 'ladder-row';
    const words = count === 1 ? `one scheme, paying with ${listWords(factors)}`
      : count === 2 ? `two schemes, both paying with ${listWords(factors)}` : `${count === 3 ? 'three' : count} schemes, all paying with ${listWords(factors)}`;
    row.innerHTML = `<div class="ladder-set"><p class="ladder-name">${listWords(spec.set)}</p><p class="ladder-count">${words}</p><div class="ladder-schemes"></div></div>${RUNGS.map((r) => `<div class="rung ${answers[r.key] ? 'is-yes' : 'is-no'}"><span class="rung-name">${r.name}</span><span class="rung-answer">${answers[r.key] ? 'yes' : 'no'}</span></div>`).join('')}`;
    const box = row.querySelector('.ladder-schemes');
    schemes.forEach((scheme, i) => {
      const cell = document.createElement('div');
      cell.className = 'ladder-cell';
      cell.style.setProperty('--w', spec.w);
      const pays = [...scheme].map(([x, f]) => `${x} pays with ${f}`).join(', ');
      marks.push(drawScheme(cell, spec, scheme, `${listWords(spec.set)}, scheme ${i + 1} of ${count}: ${pays}.`));
      box.append(cell);
    });
    grid.append(row);
  }
  return {
    highlight: (n, on) => {
      for (const s of marks) s.highlight(n, on);
    },
    goState: () => {},
  };
}
