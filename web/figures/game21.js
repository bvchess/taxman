// Game 21, from the pot to the game played: one flat figure in nine named states, the last of them the game itself,
// played move by move.  Every number keeps its mark from the pot to the game.  A page shows the states it needs, in
// this order: chapter 3 the first four (the pot, the layers, the sources, descending greedy), chapter 4 the last five
// (layered, the frames, the freedoms, the promotion, the game), and the summary all but layered, eight states.
// Layered is greedy's end again, without its scan, so that chapter 4 opens on the board and the glide to the frames
// is one step.
//
// Every fact comes from js/model.js and the recorded game of data/small.json: the layers (omega), the
// lines (maximalFactors), the sources, the frames (frameModel), what descending greedy keeps, the freedoms, the
// candidates, the promotion and the game with its taxes (replay).  Only the places are written by hand, in the layout
// of the wiki's walkthrough of game 21, and they are checked against the model, so a change the layout does not fit
// fails loudly.
//
// URL parameter (fig21 unless the page names another): k opens the page's state k, from 1; where the page plays the
// game, its last state k plays it through, and k.3 opens it after its third move.

import * as M from '../js/model.js';
import { SHAPE, listWords, numberWord } from '../js/vocab.js';
import { scene, timeline } from '../js/scene.js';
import { holdHeight } from './hold-height.js';

const N = 21;
const HALF = N / 2;
const W = 556;
const H = 410;
const CENTER_X = W / 2;

/** The named states, in order, and the title of each. */
export const STOPS = ['pot', 'layers', 'sources', 'greedy', 'layered', 'frames', 'freedoms', 'promote', 'play'];
const TITLES = {
  pot: 'The pot', layers: 'Layers', sources: 'Sources and factors', greedy: 'Descending greedy', layered: 'Layers',
  frames: 'Frames', freedoms: 'Freedoms and candidates', promote: 'The promotion', play: 'The game',
};
// The word for a state on the timeline and on the links that open it: its name, but layers for layered, which is
// never on a page with the layers state.
const stopLabel = (name) => (name === 'layered' ? 'layers' : name);

// ---------------------------------------------------------------------------------------------------------------
// Places, by hand, in the wiki's layout: the layers are columns, layer 0 at the right and the last layer at the
// left, so every line runs rightward from a number to its maximal factors, toward the front; and the frames stand
// side by side in the same order, each with its sources in a column at its left and its factors at its right.
// ---------------------------------------------------------------------------------------------------------------

// The pot: the numbers in order, seven a row.
const potAt = (m) => [CENTER_X + (((m - 1) % 7) - 3) * 58, 112 + Math.floor((m - 1) / 7) * 56];

// The layers: one column a layer, layer 0 at the right and layer 4 at the left, each ascending from the top and
// centered, so that once the columns are cut at n/2 the factors stand above the line and the sources, the top half,
// below it.
const COLUMN_X = [516, 397, 278, 159, 40];
const MID_Y = 154; // the middle of the columns, and the line at n/2 once they are cut
const STEP = 34;
const SPLIT = 22; // once cut, how far the factors stand above the line and the sources below it

// The frames, side by side.  Rows are 48 apart, and each factor stands level with the factor it divides (8 with 4,
// 4 with 2, 9 with 3, 7 with 1), so that a candidate's dotted link and its hop run straight across, between the
// sources they pass.
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
// After the promotion the moves of frame 0 make room for 9, level with 3, which pays for it: a short hop
// rightward from the factors of frame 1.
const PROMOTED_AT = { 14: [317, row(1.2)], 21: [317, row(2.1)], 9: [317, row(3)], 15: [317, row(3.9)] };
const NAME_Y = 302; // the names of the layers and the frames, under them (a frame's name from the freedoms state on)
// The lines of a frame's arithmetic, under its name: three, as two run into the next frame at a width of 390 px.
const ARITH_LINES = 3;
const arithY = (i) => NAME_Y + 20 + 18 * i;
const SCORE_Y = 352;
const SCORE_LOW = arithY(ARITH_LINES - 1) + 34; // the score where a frame's arithmetic stands above it
const boxMiddle = (layer) => BOX_AT[layer].x + BOX_AT[layer].w / 2;

// ---------------------------------------------------------------------------------------------------------------
// The facts, from the model
// ---------------------------------------------------------------------------------------------------------------

/** The facts of game 21 that the figure and its page state: the recorded game, the frames, U* and what it scores,
 *  the candidates, the promotion and the final tally. */
export function facts21(small) {
  const game = small.find((g) => g.n === N);
  const frames = M.frameModel(N);
  const uStar = M.descendingGreedy(N);
  const kept = new Set(uStar);
  const open = frames.filter((f) => !f.closed);
  const candidates = M.candidates(frames);
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
  const ends = [frames[0], frames[frames.length - 1]];
  if (open.length !== 2 || !ends.every((fr) => fr.closed) || open.some((fr) => fr.freedoms !== 1)) {
    throw new Error('game 21: the freedoms caption expects a closed frame at each end and two frames with one freedom');
  }
  // Each frame's count: its factors against its moves (what greedy kept), and the same once the promotions are made,
  // where a promoted factor leaves its frame's factors and becomes one more move of the frame it joins.
  const count = new Map();
  for (const fr of frames) {
    if (fr.factors.length - fr.kept.length !== fr.freedoms) {
      throw new Error(`game 21: frame ${fr.layer} miscounts its freedoms`);
    }
    count.set(fr.layer, { factors: fr.factors.length, moves: fr.kept.length });
  }
  const after = new Map([...count].map(([layer, c]) => [layer, { ...c }]));
  for (const x of game.promotions) {
    after.get(frames.find((fr) => !fr.closed && fr.factors.includes(x)).layer).factors--;
    const into = frames.find((fr) => fr.number === M.promotedInto(frames, x));
    if (!into) throw new Error(`game 21: the promotion of ${x} joins no frame`);
    after.get(into.layer).moves++;
  }
  for (const x of game.promotions) if (!PROMOTED_AT[x]) throw new Error(`game 21: no place for the promotion of ${x}`);
  for (const t of turns) if (t.tax.length !== 1) throw new Error(`game 21: move ${t.move} takes ${t.tax.length} numbers`);
  const scan = M.sources(N).reverse(); // descending greedy's order
  return {
    game, frames, uStar, kept, open, candidates, turns, rest, frameOf, promoted: game.promotions, score: game.score,
    inTheBag, player: sum(played.player), taxman: sum(played.taxman), scan, count, after,
    keptDown: scan.filter((x) => kept.has(x)), rejectedDown: scan.filter((x) => !kept.has(x)),
  };
}

// ---------------------------------------------------------------------------------------------------------------
// The states
// ---------------------------------------------------------------------------------------------------------------

const numbers = Array.from({ length: N }, (_, i) => i + 1);
const isSource = (x) => x > HALF;
const shapeOf = (x) => (isSource(x) ? SHAPE.circle : SHAPE.square);
const allEdges = numbers.flatMap((x) => M.maximalFactors(x).map((f) => [x, f]));
// The dotted links: from each candidate to each of its maximal factors, which all lie in the frame in front.
const candidateLinks = (f) => allEdges.filter(([a]) => f.candidates.includes(a));
// The factor that pays for each move.
const payers = (f) => new Map(f.turns.map((t) => [t.move, t.tax[0]]));
// The payments of the closed frames, [move, tax], in the order the frames stand, left to right.
const closedPayments = (f) => {
  const payOf = payers(f);
  return f.frames.filter((fr) => fr.closed).sort((a, b) => BOX_AT[a.layer].x - BOX_AT[b.layer].x)
    .flatMap((fr) => fr.kept.map((m) => [m, payOf.get(m)]));
};

// Each layer's column, ascending from the top; once split at n/2, the factors stand above the line, the largest
// nearest it, and the sources below it, the smallest nearest it.
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

const plural = (n, word) => `${n} ${word}${n === 1 ? '' : 's'}`;
/** A frame's count in words, a line each: "4 factors", "− 3 moves", "= 1 freedom". */
const arithmetic = ({ factors, moves }) => [
  plural(factors, 'factor'), `− ${plural(moves, 'move')}`, `= ${plural(factors - moves, 'freedom')}`,
];
// The marks of a frame's arithmetic: kind 'sum' is its count with U* alone, 'then' its count after the promotion.
const arithId = (kind, layer, i) => `${kind}${layer}.${i}`;

/** Every state, by name; the game is a list of states, its start, one a move and the end. */
function buildStates(f) {
  const pos = (at) => ({ x: at[0], y: at[1] });
  const closed = new Set(f.frames.filter((fr) => fr.closed).flatMap((fr) => [...fr.sources, ...fr.factors]));
  const promotedSet = new Set(f.promoted);
  const placeAfter = (x) => PROMOTED_AT[x] || FRAME_AT[x];
  const layerAt = layersAt(false);
  const splitAt = layersAt(true);
  const scanSlot = (x) => {
    const i = f.scan.indexOf(x);
    return [0.04 + i * 0.078, 0.04 + i * 0.078 + 0.14];
  };
  const frame1 = f.frames.find((fr) => fr.layer === 1);
  const firstKept = Math.max(...frame1.kept);
  const payOf = payers(f);
  const firstPay = `p${firstKept}-${payOf.get(firstKept)}`;
  const decided = (x) => (isSource(x) ? { kept: f.kept.has(x) ? 1 : 0, dashed: f.kept.has(x) ? 0 : 1 } : {});
  const S = {};

  // The pot.
  S.pot = { potName: { x: CENTER_X, y: 50 } };
  for (const x of numbers) S.pot[x] = { ...pos(potAt(x)), shape: SHAPE.plain };

  // Layers: each number in the column of its layer, joined to its maximal factors; 12 and its two are named.
  const layers = { _duration: 1300 };
  for (const x of numbers) {
    const k = M.omega(x);
    layers[x] = { ...pos(layerAt[x]), shape: SHAPE.plain, halo: [12, 6, 4].includes(x) ? 1 : 0, at: [0.06 * k, 0.06 * k + 0.55] };
  }
  for (const [a, b] of allEdges) layers[`e${a}-${b}`] = { halo: a === 12 ? 1 : 0, at: [0.55, 1] };
  for (let k = 0; k <= 4; k++) layers[`layer${k}`] = { x: COLUMN_X[k], y: NAME_Y, at: [0.5, 1] };
  S.layers = layers;

  // Sources and factors: each column cut at n/2, the factors above the line and the sources below it.  The line is
  // named at its left end, below it; the words for its two sides sit clear of the one line that crosses there.
  const split = { _duration: 1100 };
  for (const x of numbers) split[x] = { ...pos(splitAt[x]), shape: shapeOf(x), halo: isSource(x) ? 1 : 0 };
  for (const [a, b] of allEdges) split[`e${a}-${b}`] = { faded: isSource(a) ? 0 : 0.6 };
  for (let k = 0; k <= 4; k++) split[`layer${k}`] = { x: COLUMN_X[k], y: NAME_Y };
  split.rule = {};
  split.half = { x: 2, y: MID_Y - 6 };
  split.sourcesName = { x: 100, y: MID_Y + 38 };
  split.factorsName = { x: 100, y: MID_Y - 34 };
  S.sources = split;

  // Descending greedy, from 21 down, where the sources stand: each kept (gray) or rejected (hollow, dashed) in its
  // turn, with a brief halo as the scan reaches it.  1 pays for 19, the first prime kept, and the lines of a rejected
  // source go out of focus.
  const greedy = {};
  for (const x of numbers) {
    greedy[x] = { ...pos(splitAt[x]), shape: shapeOf(x), ...decided(x), ...(isSource(x) ? { at: scanSlot(x), via: { halo: 1 } } : {}) };
  }
  for (const [a, b] of allEdges) {
    const spec = { ...split[`e${a}-${b}`] };
    greedy[`e${a}-${b}`] = isSource(a) && !f.kept.has(a) ? { ...spec, faded: 1, at: scanSlot(a) } : spec;
  }
  greedy[firstPay] = { at: scanSlot(firstKept) };
  for (const key of ['rule', 'half', 'sourcesName', 'factorsName']) greedy[key] = { ...split[key] };
  for (let k = 0; k <= 4; k++) greedy[`layer${k}`] = { ...split[`layer${k}`] };
  greedy.scoreU = { x: CENTER_X, y: SCORE_Y, at: [0.85, 1] };
  S.greedy = greedy;

  // Layered: greedy's end again, without its scan, so that the glide to the frames is one step both ways.
  const layered = {};
  for (const [id, { at, via, ...look }] of Object.entries(greedy)) layered[id] = look;
  S.layered = layered;

  // Frames: the board comes apart one frame at a time, the frame in front (layer 1, at the right) first.  A frame's
  // numbers move together, and its box and the payment of a closed frame are drawn as they arrive; the names of the
  // layers, the rule and its words, and the lines of the factors, which the frames do not draw, fade out early.
  const framesView = { _duration: 2400 };
  const frameStart = (layer) => 0.2 * (layer - 1);
  for (const x of numbers) {
    const start = frameStart(f.frameOf.get(x).layer);
    framesView[x] = { ...layered[x], ...pos(FRAME_AT[x]), at: [start, start + 0.4] };
  }
  for (const [a, b] of allEdges) {
    const id = `e${a}-${b}`;
    framesView[id] = isSource(a) ? { ...layered[id] } : { ...layered[id], opacity: 0, at: [0, 0.2] };
  }
  for (const [move, tax] of closedPayments(f)) {
    const id = `p${move}-${tax}`;
    const start = frameStart(f.frameOf.get(move).layer);
    framesView[id] = id === firstPay ? { ...layered[firstPay] } : { at: [start + 0.2, start + 0.4] };
  }
  for (const key of ['rule', 'half', 'sourcesName', 'factorsName', ...[0, 1, 2, 3, 4].map((k) => `layer${k}`)]) {
    framesView[key] = { ...layered[key], opacity: 0, at: [0, 0.15] };
  }
  for (const fr of f.frames) {
    const start = frameStart(fr.layer);
    framesView[`box${fr.layer}`] = { ...BOX_AT[fr.layer], at: [start + 0.2, start + 0.4] };
  }
  framesView.scoreU = { ...layered.scoreU };
  S.frames = framesView;

  // Freedoms: the closed frames drop out of focus; the candidates take a second outline.
  const freedoms = { _duration: 1100 };
  for (const x of numbers) {
    const isCandidate = f.candidates.includes(x);
    freedoms[x] = {
      ...framesView[x], at: undefined,
      faded: closed.has(x) ? 1 : 0, opacity: closed.has(x) ? 0.6 : 1, ring: isCandidate ? 1 : 0, halo: isCandidate ? 1 : 0,
    };
  }
  for (const [a, b] of allEdges) {
    const id = `e${a}-${b}`;
    if (isSource(a)) freedoms[id] = { faded: closed.has(a) || !f.kept.has(a) ? 1 : 0, opacity: closed.has(a) ? 0.6 : 1 };
  }
  for (const [a, b] of candidateLinks(f)) freedoms[`l${a}-${b}`] = { halo: 1 };
  for (const [move, tax] of closedPayments(f)) freedoms[`p${move}-${tax}`] = { faded: 1, opacity: 0.6 };
  for (const fr of f.frames) {
    freedoms[`box${fr.layer}`] = { ...BOX_AT[fr.layer], dashed: fr.closed ? 1 : 0, faded: fr.closed ? 1 : 0 };
    freedoms[`name${fr.layer}`] = { x: boxMiddle(fr.layer), y: NAME_Y, opacity: fr.closed ? 0.6 : 1 };
    arithmetic(f.count.get(fr.layer)).forEach((_, i) => {
      freedoms[arithId('sum', fr.layer, i)] = { x: boxMiddle(fr.layer), y: arithY(i), opacity: fr.closed ? 0.6 : 1 };
    });
  }
  freedoms.scoreU = { x: CENTER_X, y: SCORE_LOW };
  S.freedoms = freedoms;

  // The promotion: 9 rounds into a circle as it hops rightward, from the factors of frame 1 into the
  // moves of frame 0, where 3 pays for it.  Its old place keeps a dashed ghost.
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
  }
  for (const [a, b] of candidateLinks(f)) {
    promote[`l${a}-${b}`] = promotedSet.has(a) ? { opacity: 0, at: [0, 0.25] } : { faded: 1, opacity: 0.4 };
  }
  for (const [move, tax] of closedPayments(f)) promote[`p${move}-${tax}`] = { ...freedoms[`p${move}-${tax}`] };
  for (const fr of f.frames) {
    promote[`box${fr.layer}`] = { ...freedoms[`box${fr.layer}`] };
    promote[`name${fr.layer}`] = { ...freedoms[`name${fr.layer}`] };
    const touched = f.count.get(fr.layer).factors !== f.after.get(fr.layer).factors ||
      f.count.get(fr.layer).moves !== f.after.get(fr.layer).moves;
    arithmetic(f.count.get(fr.layer)).forEach((_, i) => {
      const id = arithId('sum', fr.layer, i);
      // The old count goes out before the new one comes in, so the two never overlap.
      promote[id] = touched ? { ...freedoms[id], opacity: 0, at: [0, 0.3] } : { ...freedoms[id] };
      if (touched) promote[arithId('then', fr.layer, i)] = { x: boxMiddle(fr.layer), y: arithY(i), at: [0.4, 0.8] };
    });
  }
  promote.scoreU = { x: CENTER_X, y: SCORE_LOW, opacity: 0, at: [0, 0.4] };
  promote.scoreP = { x: CENTER_X, y: SCORE_LOW, at: [0.5, 0.9] };
  S.promote = promote;

  // The game: every frame back in focus, every payment drawn; then the moves, from the right.  Each turn makes a
  // move the player's (blue) and its tax the taxman's (orange), with a tag for its place in the game, and the last
  // turn wears a halo.  At the end the taxman takes the rest.
  const start = { _duration: 900 };
  for (const x of numbers) {
    start[x] = { ...pos(placeAfter(x)), shape: promotedSet.has(x) ? SHAPE.circle : shapeOf(x), ...decided(x), ...(promotedSet.has(x) ? { kept: 1 } : {}) };
  }
  for (const [a, b] of allEdges) if (isSource(a) && !promotedSet.has(b)) start[`e${a}-${b}`] = { faded: 1, opacity: 0.7 };
  for (const t of f.turns) start[`p${t.move}-${t.tax[0]}`] = {};
  for (const fr of f.frames) {
    start[`box${fr.layer}`] = { ...BOX_AT[fr.layer], dashed: fr.closed ? 1 : 0 };
    start[`name${fr.layer}`] = { x: boxMiddle(fr.layer), y: NAME_Y };
  }
  // The tally, each side with the mark of its color, so that the color is never alone.
  start.keyPlayer = { x: CENTER_X - 112, y: SCORE_Y, shape: SHAPE.circle, scale: 0.42, player: 1 };
  start.tallyPlayer = { x: CENTER_X - 99, y: SCORE_Y };
  start.keyTaxman = { x: CENTER_X + 14, y: SCORE_Y, shape: SHAPE.square, scale: 0.42, taxman: 1 };
  start.tallyTaxman = { x: CENTER_X + 27, y: SCORE_Y };
  const play = [start];
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
    play.push(s);
    prev = s;
  });
  const end = structuredClone(prev);
  end._duration = 1000;
  for (const x of numbers) end[x].halo = 0;
  for (const key of Object.keys(end)) if (key.startsWith('p') && end[key]) end[key].halo = 0;
  f.rest.forEach((x, j) => (end[x] = { ...end[x], taxman: 1, halo: 1, at: [0.15 * j, 0.15 * j + 0.6] }));
  play.push(end);
  S.play = play;
  return S;
}

// ---------------------------------------------------------------------------------------------------------------
// The words under the figure
// ---------------------------------------------------------------------------------------------------------------

/** The payments in words: "8 pays for 16 and 1 for 19". */
const paysFor = (payments) =>
  payments.map(([move, tax], i) => `${tax} ${i === 0 ? 'pays ' : ''}for ${move}`).join(' and ');

function captions(f) {
  const half = Math.floor(HALF);
  const fp = f.frameOf.get(f.promoted[0]);
  const closed = f.frames.filter((fr) => fr.closed);
  const payer = f.turns.find((t) => t.move === f.promoted[0]).tax[0];
  const f1 = f.frames.find((fr) => fr.layer === 1);
  return {
    pot: `The pot holds the numbers 1 to ${N}.`,
    layers: 'Each number stands in the column of its layer, its count of prime factors, with layer 1 at the right. A line runs rightward from each number to each of its maximal factors: 12 to 6 and 4.',
    sources: `Each column cut at n/2. Below the line are the sources, ${M.sources(N)[0]} to ${N}: they divide no other number on the board, so each is a move or nothing. Above it are the factors, 1 to ${half}.`,
    greedy: `From ${N} down, descending greedy keeps ${listWords(f.keptDown)}, each still playable with those kept before it. It rejects ${listWords(f.rejectedDown)}: their one maximal factor, 1, already pays for ${f1.kept[0]}. U* scores ${f.inTheBag}.`,
    layered: 'Game 21 as descending greedy left it, in its layers cut at n/2: the sources it kept in gray, those it ' +
      'rejected dashed. The lines of each source run to factors one layer down, in the column to its right.',
    frames:
      `The ${numberWord(f.frames.length)} frames side by side: each source with the factors ` +
      `that can pay for it, greedy's choice in gray, the sources at the left of each frame and its factors ` +
      `at the right. Each frame stands to the left of the frame in front of it. ` +
      `Thick lines mark the payments already settled: ${paysFor(closedPayments(f))}.`,
    freedoms:
      `The ${numberWord(closed.length)} frames at the ends are closed, with as many factors as moves, and drop out. ` +
      `The other ${numberWord(f.open.length)} have one freedom each: frame ${f.open[0].number} at the right and ` +
      `frame ${f.open[1].number} behind it, numbered from the front. The candidates are ${listWords(f.candidates)}, ` +
      `with a dotted line from each to its maximal factor in the frame in front.`,
    promote:
      `${f.promoted[0]} hops rightward: it leaves the factors of frame ${fp.number} and becomes a move of ` +
      `frame ${M.promotedInto(f.frames, f.promoted[0])}, paid for by ${payer}. The freedoms allow only one promotion, and ` +
      `${f.promoted[0]} is the larger. The score: ${f.inTheBag} + ${f.promoted.join(' + ')} = ${f.score}.`,
  };
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

/** Mount the figure in host, which holds .fig-state, an <svg>, .fig-timeline and .fig-caption.  states names the
 *  states the page shows, in the order of STOPS; param is the URL parameter that opens the figure at a state.  The
 *  paragraphs with data-follows set to the host's id and a data-state that lists k (one number, or several separated
 *  by spaces, such as "1 2") are marked while the figure is at its state k. */
export function mountGame21(host, { small, states = STOPS, param = 'fig21', label } = {}) {
  const names = [...states];
  const order = names.map((name) => STOPS.indexOf(name));
  if (order.some((i, j) => i < 0 || (j > 0 && i <= order[j - 1]))) {
    throw new Error(`game 21: the states ${names.join(', ')} are not states of the figure in its order`);
  }
  const f = facts21(small);
  const s = scene(host.querySelector('svg'), {
    width: W, height: H, className: 'is-dense',
    label: label ?? 'Game 21, a state at a time. The current state is described under the figure.',
  });
  for (const x of numbers) s.number(x, x);
  for (const x of f.promoted) {
    s.number(`ghost${x}`, x);
    s.line(`from${x}`, `ghost${x}`, x, 'hop');
  }
  for (const [a, b] of allEdges) {
    s.line(`e${a}-${b}`, a, b, 'thin');
  }
  for (const [a, b] of candidateLinks(f)) s.line(`l${a}-${b}`, a, b, 'link');
  f.turns.forEach((t, i) => {
    s.line(`p${t.move}-${t.tax[0]}`, t.move, t.tax[0], 'pay');
    s.tag(`tag${i + 1}`, i + 1);
  });
  for (const fr of f.frames) s.box(`box${fr.layer}`);
  s.text('potName', `the pot: the numbers 1 to ${N}`, { kind: 'name', anchor: 'middle' });
  for (let layer = 0; layer <= 4; layer++) s.text(`layer${layer}`, `layer ${layer}`, { kind: 'name', anchor: 'middle' });
  for (const fr of f.frames) {
    s.text(`name${fr.layer}`, fr.closed ? 'closed' : `frame ${fr.number}`, { kind: 'name', anchor: 'middle' });
  }
  s.line('rule', { x: 12, y: MID_Y }, { x: W - 12, y: MID_Y }, 'rule');
  s.text('half', 'n/2');
  s.text('sourcesName', 'sources', { anchor: 'middle' });
  s.text('factorsName', 'factors', { anchor: 'middle' });
  s.text('scoreU', `U* scores ${f.inTheBag}`, { kind: 'score', anchor: 'middle' });
  s.text('scoreP', `${f.inTheBag} + ${f.promoted.join(' + ')} = ${f.score}`, { kind: 'score', anchor: 'middle' });
  for (const fr of f.frames) {
    const lines = { sum: arithmetic(f.count.get(fr.layer)), then: arithmetic(f.after.get(fr.layer)) };
    for (const [kind, words] of Object.entries(lines)) {
      words.forEach((line, i) => s.text(arithId(kind, fr.layer, i), line, { anchor: 'middle' }));
    }
  }

  const all = buildStates(f);
  const list = names.flatMap((name) => (name === 'play' ? all.play : [all[name]]));
  // The game, when the page plays it, is its last state, and its moves share the last stretch of the track.
  const gameStart = names.indexOf('play');
  const count = names.length;
  const last = list.length - 1;
  const turnsDone = (pos) => Math.max(0, Math.min(f.turns.length + 1, Math.round(pos) - gameStart));
  // The tally, counted from the moves played by the state the figure is nearest.
  const tally = (pos) => {
    if (gameStart < 0) return { player: 0, taxman: 0 };
    const done = turnsDone(pos);
    const turns = f.turns.slice(0, done);
    const player = turns.reduce((a, t) => a + t.move, 0);
    const taxman = turns.reduce((a, t) => a + t.tax[0], 0) + (done > f.turns.length ? f.rest.reduce((a, b) => a + b, 0) : 0);
    return { player, taxman };
  };
  s.text('tallyPlayer', (pos) => `player ${tally(pos).player}`, { kind: 'score' });
  s.text('tallyTaxman', (pos) => `taxman ${tally(pos).taxman}`, { kind: 'score', anchor: 'start' });
  s.number('keyPlayer', '');
  s.number('keyTaxman', '');
  s.states(list);

  const words = captions(f);
  const inGame = (k) => gameStart >= 0 && k >= gameStart;
  const stateNumber = (k) => (inGame(k) ? gameStart : k) + 1;
  const nameOf = (k) => names[stateNumber(k) - 1];
  const describe = (k) => (inGame(k) ? gameWords(f, k - gameStart) : words[nameOf(k)]);
  const titleBox = host.querySelector('.fig-state');
  const captionBox = host.querySelector('.fig-caption');
  holdHeight(captionBox, list.map((_, k) => describe(k)));
  timeline(s, host.querySelector('.fig-timeline'), {
    stops: names.map((name, i) => ({ at: i, name: stopLabel(name), ...(name === 'play' ? { until: last } : {}) })),
    places: Array.from({ length: list.length }, (_, k) => (inGame(k) ? gameStart + (k - gameStart) * 0.12 : k)),
    param,
    fromParam: (value) => {
      const [whole, part] = value.split('.');
      const st = Number(whole);
      if (!Number.isInteger(st) || st < 1 || st > count) return null;
      if (st - 1 !== gameStart) return st - 1;
      return part === undefined ? last : Math.min(last, gameStart + Number(part));
    },
    toParam: (k) => (!inGame(k) ? String(k + 1) : k === last ? String(gameStart + 1) : `${gameStart + 1}.${k - gameStart}`),
    describe: (k) => `${TITLES[nameOf(k)]}, state ${stateNumber(k)} of ${count}. ${describe(k)}`,
    title: (k) => {
      titleBox.querySelector('.fig-state-n').textContent = `${stateNumber(k)} of ${count}`;
      titleBox.querySelector('.fig-state-name').textContent = TITLES[nameOf(k)];
      captionBox.textContent = describe(k);
      host.dataset.state = stateNumber(k);
      for (const p of document.querySelectorAll(`[data-follows="${host.id}"]`)) {
        p.classList.toggle('is-current', p.dataset.state.split(/\s+/).includes(String(stateNumber(k))));
      }
    },
    label: 'Game 21 figure: state',
  });
  let playId = 0;
  return {
    scene: s,
    facts: f,
    stopName: (value) => stopLabel(names[Number(String(value).split('.')[0]) - 1]),
    /** Send the figure to a state named as in the URL: "2", or the game's state to play it through, or "4.3". */
    async goState(value) {
      const id = ++playId;
      const [whole, part] = String(value).split('.');
      const st = Number(whole) - 1;
      if (st !== gameStart) return s.go(st);
      if (part !== undefined) return s.go(gameStart + Number(part));
      await s.go(gameStart);
      for (let k = gameStart + 1; k <= last && id === playId; k++) {
        await new Promise((r) => setTimeout(r, 380));
        if (id !== playId || Math.round(s.pos) !== k - 1) return;
        await s.go(k);
      }
    },
    highlight: (n, on) => s.highlight(n, on),
  };
}
