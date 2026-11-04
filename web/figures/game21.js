// Game 21, from the board to the frames: one flat figure in eight named states, the last of them the game itself,
// played move by move.  Every number keeps its mark from the pot to the game.
//
// Every fact comes from js/model.js and the recorded game of data/small.json: the layers (omega), the
// lines (maximalFactors), the sources, the frames (frameModel), what descending greedy keeps, the freedoms, the
// candidates, the promotion and the game with its taxes (replay).  Only the places are written by hand, in the layout
// of the wiki's walkthrough of game 21, and they are checked against the model, so a change the layout does not fit
// fails loudly.
//
// URL parameter: fig21=k opens state k (1 to 8); fig21=8.3 opens the game after its third move.

import * as M from '../js/model.js';
import { SHAPE, fmt, listWords } from '../js/vocab.js';
import { scene, timeline } from '../js/scene.js';

const N = 21;
const HALF = N / 2;
const W = 556;
const H = 370;
const CENTER_X = W / 2;

/** The named stops of the slider, and the title of each state. */
export const STOPS = ['pot', 'layers', 'sources', 'frames', 'greedy', 'freedoms', 'promote', 'play'];
export const TITLES = [
  'The pot', 'Layers', 'Sources and factors', 'Frames', 'Descending greedy', 'Freedoms and candidates', 'The promotion',
  'The game',
];

// ---------------------------------------------------------------------------------------------------------------
// Places, by hand, in the wiki's layout: the layers are columns, layer 0 at the right and the last layer at the
// left, so every line runs rightward from a number to its maximal factors, toward the front; and the frames stand
// side by side in the same order, each with its sources in a column at its left and its factors at its right.
// ---------------------------------------------------------------------------------------------------------------

// The pot: the numbers in order, seven a row.
const potAt = (m) => [CENTER_X + (((m - 1) % 7) - 3) * 58, 112 + Math.floor((m - 1) / 7) * 56];

// The layers: one column a layer, layer 0 at the right and layer 4 at the left, each in order and centered.
const COLUMN_X = [516, 397, 278, 159, 40];
const MID_Y = 154; // the middle of the columns, and the line at n/2 in the state "sources"
const STEP = 34;
const SPLIT = 22; // in the state "sources", how far the factors stand above the line and the sources below it

// The frames, side by side.  Rows are 48 apart, and each factor stands level with the factor it divides (8 with 4,
// 4 with 2, 9 with 3, 7 with 1), so that the dotted links run straight across, between the sources they pass.
const row = (r) => 58 + 48 * r;
const FRAME_AT = {
  16: [33, row(1)], 8: [97, row(1)],
  20: [175, row(0.5)], 12: [175, row(1.5)], 18: [175, row(2.5)],
  10: [239, row(0)], 4: [239, row(1)], 6: [239, row(2)], 9: [239, row(3)],
  14: [317, row(1.5)], 21: [317, row(2.5)], 15: [317, row(3.5)],
  2: [381, row(1)], 7: [381, row(2)], 3: [381, row(3)], 5: [381, row(4)],
  11: [459, row(0.5)], 13: [459, row(1.5)], 17: [459, row(2.5)], 19: [459, row(3.5)],
  1: [523, row(2)],
};
const BOX_AT = {
  4: { x: 6, y: 30, w: 118, h: 248 },
  3: { x: 148, y: 30, w: 118, h: 248 },
  2: { x: 290, y: 30, w: 118, h: 248 },
  1: { x: 432, y: 30, w: 118, h: 248 },
};
// After the promotion the moves of the layer-2 frame make room for 9, level with 3, which pays for it: a short hop
// rightward from the factors of the layer-3 frame.
const PROMOTED_AT = { 14: [317, row(1.2)], 21: [317, row(2.1)], 9: [317, row(3)], 15: [317, row(3.9)] };
// The links to the frame in front that the figure draws: those that pass clear of other numbers.
const LINKS_DRAWN = [[8, 4], [4, 2], [9, 3], [10, 2], [7, 1]];
const NAME_Y = 302; // the names of the layers and the frames, under them
const NOTE_Y = 322; // the notes under the frames
const SCORE_Y = 352;
const boxMiddle = (layer) => BOX_AT[layer].x + BOX_AT[layer].w / 2;

// ---------------------------------------------------------------------------------------------------------------
// The facts, from the model
// ---------------------------------------------------------------------------------------------------------------

function facts(small) {
  const game = small.find((g) => g.n === N);
  const frames = M.frameModel(N);
  const uStar = M.descendingGreedy(N);
  const kept = new Set(uStar);
  const open = frames.filter((f) => !f.closed);
  // A candidate here is a factor of an open frame that can be promoted alone: U* with it is playable.
  const candidates = open.flatMap((f) => f.factors).filter((x) => M.isPlayable([...uStar, x])).sort((a, b) => a - b);
  const played = M.replay(N, game.moves);
  const turns = played.log.filter((entry) => 'move' in entry);
  const rest = played.log.find((entry) => 'rest' in entry).rest;
  const sum = (list) => list.reduce((a, b) => a + b, 0);
  const inTheBag = sum(uStar);
  if (inTheBag + sum(game.promotions) !== game.score || M.score(played) !== game.score) {
    throw new Error('game 21: U* plus the promotions does not give the recorded score');
  }
  const frameOf = new Map();
  for (const f of frames) for (const x of [...f.sources, ...f.factors]) frameOf.set(x, f);
  // The hand placement must agree with the model: every number placed, every frame given a box.
  for (const f of frames) {
    if (!BOX_AT[f.layer]) throw new Error(`game 21 has a frame in layer ${f.layer} that the figure has no box for`);
    for (const x of [...f.sources, ...f.factors]) if (!FRAME_AT[x]) throw new Error(`game 21: no place for ${x}`);
  }
  for (const x of game.promotions) if (!PROMOTED_AT[x]) throw new Error(`game 21: no place for the promotion of ${x}`);
  for (const t of turns) if (t.tax.length !== 1) throw new Error(`game 21: move ${t.move} takes ${t.tax.length} numbers`);
  return {
    game, frames, uStar, kept, open, candidates, turns, rest, frameOf, promoted: game.promotions, score: game.score,
    inTheBag, player: sum(played.player), taxman: sum(played.taxman),
  };
}

// ---------------------------------------------------------------------------------------------------------------
// The states
// ---------------------------------------------------------------------------------------------------------------

const numbers = Array.from({ length: N }, (_, i) => i + 1);
const isSource = (x) => x > HALF;
const shapeOf = (x) => (isSource(x) ? SHAPE.circle : SHAPE.square);
const allEdges = numbers.flatMap((x) => M.maximalFactors(x).map((f) => [x, f]));

function layersAt(split) {
  const columns = [[], [], [], [], []];
  for (const x of numbers) columns[M.omega(x)].push(x);
  const at = {};
  columns.forEach((column, layer) => {
    const x = COLUMN_X[layer];
    if (!split) {
      column.forEach((n, j) => (at[n] = [x, MID_Y + (j - (column.length - 1) / 2) * STEP]));
      return;
    }
    const factors = column.filter((n) => !isSource(n));
    const srcs = column.filter(isSource);
    factors.forEach((n, j) => (at[n] = [x, MID_Y - SPLIT - (factors.length - 1 - j) * STEP]));
    srcs.forEach((n, j) => (at[n] = [x, MID_Y + SPLIT + j * STEP]));
  });
  return at;
}

function buildStates(f) {
  const pos = (at) => ({ x: at[0], y: at[1] });
  const closed = new Set(f.frames.filter((fr) => fr.closed).flatMap((fr) => [...fr.sources, ...fr.factors]));
  const promotedSet = new Set(f.promoted);
  const placeAfter = (x) => PROMOTED_AT[x] || FRAME_AT[x];
  const layerAt = layersAt(false);
  const splitAt = layersAt(true);
  const scan = numbers.filter(isSource).reverse(); // descending greedy's order
  const scanSlot = (x) => {
    const i = scan.indexOf(x);
    return [0.04 + i * 0.078, 0.04 + i * 0.078 + 0.14];
  };
  const frame1 = f.frames.find((fr) => fr.layer === 1);
  const firstKept = Math.max(...frame1.kept);
  const linkDrawn = new Set(LINKS_DRAWN.map(([a, b]) => `${a}-${b}`));
  const payOf = new Map(f.turns.map((t) => [t.move, t.tax[0]]));
  const S = []; // the states, in order

  // 1. The pot.
  const pot = { potName: { x: CENTER_X, y: 50 } };
  for (const x of numbers) pot[x] = { ...pos(potAt(x)), shape: shapeOf(x) };
  S.push(pot);

  // 2. Layers: each number in the column of its layer, joined to its maximal factors; 12 and its two are named.
  const layers = { _duration: 1300 };
  for (const x of numbers) {
    const k = M.omega(x);
    layers[x] = { ...pos(layerAt[x]), shape: shapeOf(x), halo: [12, 6, 4].includes(x) ? 1 : 0, at: [0.06 * k, 0.06 * k + 0.55] };
  }
  for (const [a, b] of allEdges) layers[`e${a}-${b}`] = { halo: a === 12 ? 1 : 0, at: [0.55, 1] };
  for (let k = 0; k <= 4; k++) layers[`layer${k}`] = { x: COLUMN_X[k], y: NAME_Y, at: [0.5, 1] };
  S.push(layers);

  // 3. Sources and factors: each column cut at n/2, the factors above the line and the sources below it.
  const split = { _duration: 1100 };
  for (const x of numbers) split[x] = { ...pos(splitAt[x]), shape: shapeOf(x), halo: isSource(x) ? 1 : 0 };
  for (const [a, b] of allEdges) split[`e${a}-${b}`] = { faded: isSource(a) ? 0 : 0.6 };
  for (let k = 0; k <= 4; k++) split[`layer${k}`] = { x: COLUMN_X[k], y: NAME_Y };
  split.rule = {};
  // The line at n/2 is named at its left end; the words for its two sides sit where only one line crosses.
  split.half = { x: 2, y: MID_Y - 9 };
  split.factorsName = { x: (COLUMN_X[3] + COLUMN_X[4]) / 2, y: MID_Y - 13 };
  split.sourcesName = { x: (COLUMN_X[3] + COLUMN_X[4]) / 2, y: MID_Y + 13 };
  S.push(split);

  // 4. Frames: each source with the factors that can pay for it, frame by frame, the frame in front at the right.
  const framesView = { _duration: 1500 };
  for (const x of numbers) {
    const k = f.frameOf.get(x).layer;
    framesView[x] = { ...pos(FRAME_AT[x]), shape: shapeOf(x), at: [0.1 * (4 - k), 0.1 * (4 - k) + 0.6] };
  }
  for (const [a, b] of allEdges) {
    if (isSource(a)) framesView[`e${a}-${b}`] = { at: [0.5, 1] };
    if (!isSource(a) && linkDrawn.has(`${a}-${b}`)) framesView[`l${a}-${b}`] = { at: [0.6, 1] };
  }
  for (const fr of f.frames) {
    framesView[`box${fr.layer}`] = { ...BOX_AT[fr.layer], at: [0.45, 1] };
    framesView[`layer${fr.layer}`] = { x: boxMiddle(fr.layer), y: NAME_Y };
  }
  S.push(framesView);

  // 5. Descending greedy, from 21 down: each source kept (gray) or rejected (hollow, dashed) in its turn, with a
  // brief halo as the scan reaches it.  1 pays for 19, the first prime kept.
  const greedy = {};
  for (const x of numbers) {
    const decided = isSource(x) ? { kept: f.kept.has(x) ? 1 : 0, dashed: f.kept.has(x) ? 0 : 1 } : {};
    greedy[x] = { ...pos(FRAME_AT[x]), shape: shapeOf(x), ...decided, ...(isSource(x) ? { at: scanSlot(x), via: { halo: 1 } } : {}) };
  }
  for (const [a, b] of allEdges) {
    if (isSource(a)) greedy[`e${a}-${b}`] = isSource(a) && !f.kept.has(a) ? { faded: 1, at: scanSlot(a) } : {};
    if (!isSource(a) && linkDrawn.has(`${a}-${b}`)) greedy[`l${a}-${b}`] = {};
  }
  greedy[`p${firstKept}-${payOf.get(firstKept)}`] = { at: scanSlot(firstKept) };
  for (const fr of f.frames) {
    greedy[`box${fr.layer}`] = { ...BOX_AT[fr.layer] };
    greedy[`layer${fr.layer}`] = { x: boxMiddle(fr.layer), y: NAME_Y };
  }
  greedy.scoreU = { x: CENTER_X, y: SCORE_Y, at: [0.85, 1] };
  S.push(greedy);

  // 6. Freedoms: the closed frames drop out of focus; the candidates take a second outline.
  const freedoms = { _duration: 1100 };
  for (const x of numbers) {
    const isCandidate = f.candidates.includes(x);
    freedoms[x] = {
      ...greedy[x], at: undefined, via: undefined,
      faded: closed.has(x) ? 1 : 0, opacity: closed.has(x) ? 0.6 : 1, ring: isCandidate ? 1 : 0, halo: isCandidate ? 1 : 0,
    };
  }
  for (const [a, b] of allEdges) {
    const id = `e${a}-${b}`;
    if (isSource(a)) freedoms[id] = { faded: closed.has(a) || !f.kept.has(a) ? 1 : 0, opacity: closed.has(a) ? 0.6 : 1 };
    const lid = `l${a}-${b}`;
    if (!isSource(a) && linkDrawn.has(`${a}-${b}`)) {
      freedoms[lid] = f.candidates.includes(a) ? { halo: 1 } : { faded: 1, opacity: 0.5 };
    }
  }
  freedoms[`p${firstKept}-${payOf.get(firstKept)}`] = { faded: 1, opacity: 0.6 };
  for (const fr of f.frames) {
    freedoms[`box${fr.layer}`] = { ...BOX_AT[fr.layer], dashed: fr.closed ? 1 : 0, faded: fr.closed ? 1 : 0 };
    freedoms[`layer${fr.layer}`] = { x: boxMiddle(fr.layer), y: NAME_Y, opacity: fr.closed ? 0.6 : 1 };
    freedoms[`free${fr.layer}`] = { x: boxMiddle(fr.layer), y: NOTE_Y, opacity: fr.closed ? 0.6 : 1 };
  }
  freedoms.scoreU = { x: CENTER_X, y: SCORE_Y };
  S.push(freedoms);

  // 7. The promotion: 9 rounds into a circle as it hops rightward, from the factors of the layer-3 frame into the
  // moves of the layer-2 frame, where 3 pays for it.  Its old place keeps a dashed ghost.
  const promote = { _duration: 1600 };
  for (const x of numbers) {
    promote[x] = {
      ...freedoms[x],
      ...pos(placeAfter(x)),
      halo: promotedSet.has(x) || [...promotedSet].some((p) => payOf.get(p) === x) ? 1 : 0,
      ...(promotedSet.has(x) ? { shape: SHAPE.circle, kept: 1, ring: 1, at: [0.1, 0.75] } : {}),
      ...(f.candidates.includes(x) && !promotedSet.has(x) ? { ring: 0.35, faded: 0.5 } : {}),
    };
    if (!promotedSet.has(x) && PROMOTED_AT[x]) promote[x].at = [0, 0.45];
  }
  for (const x of f.promoted) {
    promote[`ghost${x}`] = { ...pos(FRAME_AT[x]), shape: SHAPE.square, dashed: 1, faded: 1, at: [0.15, 0.6] };
    promote[`from${x}`] = { faded: 0.3, at: [0.45, 0.9] };
    promote[`p${x}-${payOf.get(x)}`] = { halo: 1, at: [0.55, 1] };
  }
  for (const [a, b] of allEdges) {
    const id = `e${a}-${b}`;
    if (isSource(a)) promote[id] = promotedSet.has(b) ? { opacity: 0, at: [0, 0.3] } : { ...freedoms[id] };
    const lid = `l${a}-${b}`;
    if (freedoms[lid] && f.candidates.includes(a)) {
      promote[lid] = promotedSet.has(a) ? { opacity: 0, at: [0, 0.25] } : { faded: 1, opacity: 0.4 };
    } else if (freedoms[lid]) promote[lid] = { ...freedoms[lid] };
  }
  promote[`p${firstKept}-${payOf.get(firstKept)}`] = { ...freedoms[`p${firstKept}-${payOf.get(firstKept)}`] };
  for (const fr of f.frames) {
    promote[`box${fr.layer}`] = { ...freedoms[`box${fr.layer}`] };
    promote[`layer${fr.layer}`] = { ...freedoms[`layer${fr.layer}`] };
    if (fr.closed) promote[`free${fr.layer}`] = { ...freedoms[`free${fr.layer}`] };
    else {
      // The count of freedoms goes out before the words that replace it come in, so the two never overlap.
      promote[`free${fr.layer}`] = { x: boxMiddle(fr.layer), y: NOTE_Y, opacity: 0, at: [0, 0.3] };
      promote[`left${fr.layer}`] = { x: boxMiddle(fr.layer), y: NOTE_Y, at: [0.4, 0.8] };
    }
  }
  promote.scoreU = { x: CENTER_X, y: SCORE_Y, opacity: 0, at: [0, 0.4] };
  promote.scoreP = { x: CENTER_X, y: SCORE_Y, at: [0.5, 0.9] };
  S.push(promote);

  // 8. The game: every frame back in focus, every payment drawn; then the moves, from the right.  Each turn makes a
  // move the player's (blue) and its tax the taxman's (orange), with a tag for its place in the game, and the last
  // turn wears a halo.  At the end the taxman takes the rest.
  const start = { _duration: 900 };
  for (const x of numbers) {
    const decided = isSource(x) ? { kept: f.kept.has(x) ? 1 : 0, dashed: f.kept.has(x) ? 0 : 1 } : {};
    start[x] = { ...pos(placeAfter(x)), shape: promotedSet.has(x) ? SHAPE.circle : shapeOf(x), ...decided, ...(promotedSet.has(x) ? { kept: 1 } : {}) };
  }
  for (const [a, b] of allEdges) if (isSource(a) && !promotedSet.has(b)) start[`e${a}-${b}`] = { faded: 1, opacity: 0.7 };
  for (const t of f.turns) start[`p${t.move}-${t.tax[0]}`] = {};
  for (const fr of f.frames) {
    start[`box${fr.layer}`] = { ...BOX_AT[fr.layer], dashed: fr.closed ? 1 : 0 };
    start[`layer${fr.layer}`] = { x: boxMiddle(fr.layer), y: NAME_Y };
  }
  // The tally, each side with the mark of its color, so that the color is never alone.
  start.keyPlayer = { x: CENTER_X - 112, y: SCORE_Y, shape: SHAPE.circle, scale: 0.42, player: 1 };
  start.tallyPlayer = { x: CENTER_X - 99, y: SCORE_Y };
  start.keyTaxman = { x: CENTER_X + 14, y: SCORE_Y, shape: SHAPE.square, scale: 0.42, taxman: 1 };
  start.tallyTaxman = { x: CENTER_X + 27, y: SCORE_Y };
  S.push(start);
  let prev = start;
  f.turns.forEach((t, i) => {
    const s = structuredClone(prev);
    s._duration = 700;
    for (const x of numbers) s[x].halo = 0;
    for (const key of Object.keys(s)) if (key.startsWith('p') && s[key]) s[key].halo = 0;
    s[t.move] = { ...s[t.move], player: 1, kept: 0, halo: 1, at: [0, 0.6] };
    s[t.tax[0]] = { ...s[t.tax[0]], taxman: 1, halo: 1, at: [0.3, 0.9] };
    s[`p${t.move}-${t.tax[0]}`] = { halo: 1 };
    const [mx, my] = placeAfter(t.move);
    s[`tag${i + 1}`] = { x: mx - 15, y: my - 17, at: [0, 0.5] };
    S.push(s);
    prev = s;
  });
  const end = structuredClone(prev);
  end._duration = 1000;
  for (const x of numbers) end[x].halo = 0;
  for (const key of Object.keys(end)) if (key.startsWith('p') && end[key]) end[key].halo = 0;
  f.rest.forEach((x, j) => (end[x] = { ...end[x], taxman: 1, halo: 1, at: [0.15 * j, 0.15 * j + 0.6] }));
  S.push(end);
  return S;
}

// ---------------------------------------------------------------------------------------------------------------
// The words under the figure
// ---------------------------------------------------------------------------------------------------------------

function captions(f) {
  const f1 = f.frames.find((fr) => fr.layer === 1);
  const fp = f.frameOf.get(f.promoted[0]);
  const into = f.frames.find((fr) => fr.layer === M.omega(f.promoted[0]));
  const payer = f.turns.find((t) => t.move === f.promoted[0]).tax[0];
  return [
    `The pot holds the numbers 1 to ${N}. The circles are the numbers above n/2, the squares the rest.`,
    'Each number stands in the column of its layer, its count of prime factors, with layer 1 at the right. A line runs rightward from each number to each of its maximal factors: 12 to 6 and 4.',
    `The sources, ${M.sources(N)[0]} to ${N}, lie above n/2. They divide no other number on the board, so they are never tax: each is a move or nothing. The factors are 1 to ${Math.floor(HALF)}.`,
    `${f.frames.length} frames side by side: each source with the factors that can pay for it. A frame's sources, at its left, share a layer, and its factors, at its right, lie one layer below. A dotted arrow runs from a factor to its maximal factor in the frame in front.`,
    `Descending greedy keeps each source, from ${N} down, that is still playable with those kept. It keeps ${listWords(f1.kept)} and rejects ${listWords([...f1.rejected].reverse())}: their only factor, 1, pays for ${f1.kept[0]}. U* scores ${f.inTheBag}.`,
    `The frames of layers ${listWords(f.frames.filter((fr) => fr.closed).map((fr) => fr.layer))} are closed, with as many factors as moves, and drop out. The frames of layers ${listWords(f.open.map((fr) => fr.layer))} have one freedom each. The candidates are ${listWords(f.candidates)}.`,
    `${f.promoted[0]} hops rightward: it leaves the factors of the layer-${fp.layer} frame and becomes a move of the layer-${into.layer} frame, paid by ${payer}. Only one candidate fits, and ${f.promoted[0]} is the larger. The score: ${f.inTheBag} + ${f.promoted.join(' + ')} = ${f.score}.`,
  ];
}

function gameWords(f, done) {
  if (done === 0) return `The game itself, played from the right: the moves ${listWords(f.game.moves)}, each paid for by the factor its thick line marks.`;
  const turns = f.turns.slice(0, done);
  const player = turns.reduce((s, t) => s + t.move, 0);
  const taxman = turns.reduce((s, t) => s + t.tax[0], 0);
  if (done > f.turns.length) return `No move is left, so the taxman takes the rest, ${listWords(f.rest)}. The player scores ${f.player}, the taxman ${f.taxman}.`;
  const t = turns[done - 1];
  return `Move ${done}: the player takes ${t.move}, and the taxman takes ${t.tax[0]}. ${player} to ${taxman}.`;
}

// ---------------------------------------------------------------------------------------------------------------
// Mounting
// ---------------------------------------------------------------------------------------------------------------

/** Mount the figure in host, which holds .fig-state, an <svg>, .fig-timeline and .fig-caption. */
export function mountGame21(host, { small }) {
  const f = facts(small);
  const s = scene(host.querySelector('svg'), {
    width: W, height: H, className: 'is-dense',
    label: 'Game 21, from the board to the frames. The current state is described under the figure.',
  });
  for (const x of numbers) s.number(x, x);
  for (const x of f.promoted) {
    s.number(`ghost${x}`, x);
    s.line(`from${x}`, `ghost${x}`, x, 'link');
  }
  for (const [a, b] of allEdges) {
    s.line(`e${a}-${b}`, a, b, 'thin');
    if (!isSource(a)) s.line(`l${a}-${b}`, a, b, 'link');
  }
  f.turns.forEach((t, i) => {
    s.line(`p${t.move}-${t.tax[0]}`, t.move, t.tax[0], 'pay');
    s.tag(`tag${i + 1}`, i + 1);
  });
  for (const fr of f.frames) s.box(`box${fr.layer}`);
  s.text('potName', `the pot: the numbers 1 to ${N}`, { kind: 'name', anchor: 'middle' });
  for (let layer = 0; layer <= 4; layer++) s.text(`layer${layer}`, `layer ${layer}`, { kind: 'name', anchor: 'middle' });
  s.line('rule', { x: 12, y: MID_Y }, { x: W - 12, y: MID_Y }, 'rule');
  s.text('half', 'n/2');
  s.text('factorsName', 'factors', { anchor: 'middle' });
  s.text('sourcesName', 'sources', { anchor: 'middle' });
  s.text('scoreU', `U* scores ${f.inTheBag}`, { kind: 'score', anchor: 'middle' });
  s.text('scoreP', `${f.inTheBag} + ${f.promoted.join(' + ')} = ${f.score}`, { kind: 'score', anchor: 'middle' });
  for (const fr of f.frames) {
    s.text(`free${fr.layer}`, fr.closed ? 'closed' : `${fr.freedoms} freedom${fr.freedoms === 1 ? '' : 's'}`, { anchor: 'middle' });
    if (!fr.closed) s.text(`left${fr.layer}`, 'no freedom left', { anchor: 'middle' });
  }
  const GAME_START = STOPS.length - 1;
  // The tally, counted from the moves played by the state the figure is nearest.
  const tally = (pos) => {
    const done = Math.max(0, Math.min(f.turns.length + 1, Math.round(pos) - GAME_START));
    const turns = f.turns.slice(0, done);
    const player = turns.reduce((a, t) => a + t.move, 0);
    const taxman = turns.reduce((a, t) => a + t.tax[0], 0) + (done > f.turns.length ? f.rest.reduce((a, b) => a + b, 0) : 0);
    return { player, taxman };
  };
  s.text('tallyPlayer', (pos) => `player ${tally(pos).player}`, { kind: 'score' });
  s.text('tallyTaxman', (pos) => `taxman ${tally(pos).taxman}`, { kind: 'score', anchor: 'start' });
  s.number('keyPlayer', '');
  s.number('keyTaxman', '');
  s.states(buildStates(f));

  const last = s.count - 1;
  const words = captions(f);
  const stateNumber = (k) => Math.min(k, GAME_START) + 1;
  const describe = (k) => (k < GAME_START ? words[k] : gameWords(f, k - GAME_START));
  const titleBox = host.querySelector('.fig-state');
  const captionBox = host.querySelector('.fig-caption');
  timeline(s, host.querySelector('.fig-timeline'), {
    stops: [
      ...STOPS.slice(0, -1).map((name, i) => ({ at: i, name })),
      { at: GAME_START, until: last, name: STOPS[GAME_START] },
    ],
    // The game's ten turns share the last stretch of the track.
    places: Array.from({ length: s.count }, (_, k) => (k <= GAME_START ? k : GAME_START + (k - GAME_START) * 0.12)),
    param: 'fig21',
    fromParam: (value) => {
      const [whole, part] = value.split('.');
      const st = Number(whole);
      if (!Number.isInteger(st) || st < 1 || st > STOPS.length) return null;
      if (st < STOPS.length) return st - 1;
      return part === undefined ? last : Math.min(last, GAME_START + Number(part));
    },
    toParam: (k) => (k < GAME_START ? String(k + 1) : k === last ? String(STOPS.length) : `${STOPS.length}.${k - GAME_START}`),
    describe: (k) => `${TITLES[stateNumber(k) - 1]}, state ${stateNumber(k)} of ${STOPS.length}. ${describe(k)}`,
    title: (k) => {
      titleBox.querySelector('.fig-state-n').textContent = `${stateNumber(k)} of ${STOPS.length}`;
      titleBox.querySelector('.fig-state-name').textContent = TITLES[stateNumber(k) - 1];
      captionBox.textContent = describe(k);
      host.dataset.state = stateNumber(k);
      for (const p of document.querySelectorAll('[data-follows="fig21"]')) {
        p.classList.toggle('is-current', p.dataset.state === String(stateNumber(k)));
      }
    },
    label: 'Game 21 figure: state',
  });
  let playId = 0;
  return {
    scene: s,
    facts: f,
    stopName: (label) => STOPS[Number(String(label).split('.')[0]) - 1],
    /** Send the figure to a state named as in the URL: "4", or "8" to play the game through, or "8.3". */
    async goState(label) {
      const id = ++playId;
      const [whole, part] = String(label).split('.');
      const st = Number(whole);
      if (st < STOPS.length) return s.go(st - 1);
      if (part !== undefined) return s.go(GAME_START + Number(part));
      await s.go(GAME_START);
      for (let k = GAME_START + 1; k <= last && id === playId; k++) {
        await new Promise((r) => setTimeout(r, 380));
        if (id !== playId || Math.round(s.pos) !== k - 1) return;
        await s.go(k);
      }
    },
    highlight: (n, on) => s.highlight(n, on),
  };
}
