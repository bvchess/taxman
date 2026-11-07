// A move slips in.  Game 12 built from game 11's optimal game, which is a game of 12 as it stands: 12
// cannot come last, since every maximal factor of it is gone, so it goes just before the move that takes the last of
// them from the pot, takes that tax itself, and the move it robbed is left out.  A row of moves, each over its tax, and
// at the right what is left out of the game (figures/game-row.js), in four states: the game as it stands, the maximal
// factors of 12 that are gone, 12 before 8, and 8 left out.  A flat figure.
//
// The facts are computed: slipFacts finds where n goes in any game of n − 1 and replays the result with js/model.js,
// and the figure checks them against data/small.json: the game of n − 1 scores opt(n − 1), and the game it builds
// scores opt(n − 1) + n less the move left out.

import * as M from '../js/model.js';
import { timeline, scene } from '../js/scene.js';
import { fmt, listWords } from '../js/vocab.js';
import { holdHeight } from './hold-height.js';
import {
  HEADER_H, MOVE_Y, PAD, POT_GAP, REFERENCE, addRow, columnX, gameRow, payId, potLook, rowHeight, rowLooks, rowWidth,
} from './game-row.js';

const N = 12;
const HEADER = { name: 'moves', out: true };
const ARC = 26; // how high 12 rises on its way from the pot to its column

/** Play n into the game of n − 1 given by moves: just before the first move after which no maximal factor of n is left
 *  in the pot, or at the end if one is left (a number is playable exactly when one of its maximal factors is in the
 *  pot).  The move it goes before is left out if it can no longer be played.
 *  Returns { n, before, after, factors, last, cut, dropped, gain }: the row of the game of n − 1 as it stands in game n
 *  (null if n goes last, when that game is not over) and the row of the game built; the maximal factors of n,
 *  ascending; the last of them to go and the move that takes it (null if one stays); the move left out (null if
 *  none); and the change of score. */
export function slipFacts(n, moves) {
  const factors = M.maximalFactors(n).sort((a, b) => a - b);
  let state = M.newGame(n);
  let cut = -1;
  let last = null;
  for (let i = 0; i < moves.length && cut < 0; i++) {
    const gone = state;
    state = M.play(state, moves[i]);
    if (!factors.some((d) => state.pot.has(d))) {
      cut = i;
      last = factors.filter((d) => gone.pot.has(d)).pop();
    }
  }
  const at = cut < 0 ? moves.length : cut;
  let played = moves.slice(0, at).reduce((game, m) => M.play(game, m), M.newGame(n));
  played = M.play(played, n);
  const dropped = cut >= 0 && !M.legalMoves(played).includes(moves[cut]) ? moves[cut] : null;
  const rest = moves.slice(at).filter((m) => m !== dropped);
  const after = gameRow(n, [...moves.slice(0, at), n, ...rest]);
  const found = cut >= 0;
  return {
    n, before: found ? gameRow(n, moves) : null, after, factors, last, cut: found ? moves[cut] : null, dropped,
    gain: n - (dropped ?? 0),
  };
}

/** The facts of game n's slip from the recorded games, checked: the game of n − 1 scores opt(n − 1) and the new game
 *  opt(n − 1) + the gain.  Throws if not. */
export function slipOfRecord(small, n) {
  const f = slipFacts(n, small[n - 2].moves);
  const fail = (why) => {
    throw new Error(`slip of ${n}: ${why}`);
  };
  const score = small[n - 2].score;
  if (f.before && f.before.score !== score) fail(`game ${n - 1}'s record scores ${score}`);
  if (f.after.score !== score + f.gain) fail('the new game does not score opt(n − 1) and the gain');
  return f;
}

function words(f) {
  const { n, before, after, factors, last, cut, dropped } = f;
  const tax = after.turns.find((t) => t.move === n).tax;
  const gone = ['is', 'are both', 'are all'][Math.min(factors.length, 3) - 1];
  const plural = factors.length > 1 ? 's' : '';
  return [
    `Game ${n - 1}'s optimal game, played in game ${n}: each move takes the same tax, and it leaves ${listWords(before.rest)} `
    + `out.  It scores ${fmt(before.score)}.`,
    `${n} cannot come last: by then its maximal factor${plural}, ${listWords(factors)}, ${gone} gone.  The last to go `
    + `is ${last}, ${last === cut ? 'which the player plays as a move' : `which ${cut} takes`}.`,
    `So ${n} goes just before ${cut} and takes ${listWords(tax)}.  Now ${cut} has no maximal factor left in the pot.`,
    `We leave ${dropped} out, and ${after.moves.join(', ')} scores ${scoreWords(f)}.`,
  ];
}

const scoreWords = ({ n, before, after, dropped }) => `${fmt(before.score)} + ${n} − ${dropped} = ${fmt(after.score)}`;

// The four states: the game as it stands, the maximal factors of n gone, n before the move it robs, and that move
// left out.
function states(f) {
  const { n, before, after, factors, cut, dropped } = f;
  const height = HEADER_H + rowHeight([before, after]) + 16;
  const score = (id) => ({ [id]: { x: PAD, y: height - 10 } });
  // Six columns: the move before which n goes, and the empty one that n takes, are the last two.
  const was = [...before.moves, null];
  const slotted = [...after.moves, cut];
  const place = { top: HEADER_H, header: HEADER };
  const pot = (k) => columnX(slotted.length + POT_GAP + k);
  const taxN = after.turns.find((t) => t.move === n).tax;
  const slips = [...factors, n];
  const shifted = columnX(slotted.length - 1);
  const built = (halo) => rowLooks(after, { ...place, slots: slotted, halo: new Set(halo) });
  const first = { ...rowLooks(before, { ...place, slots: was }), ...score('score-a') };
  const second = { ...rowLooks(before, { ...place, slots: was, halo: new Set(slips) }), ...score('score-a') };
  const third = built([n, ...taxN]);
  // 8 shifts right with its tax, n arrives from the pot, then the tax passes from 8 to n.
  Object.assign(third, score('score-a'), {
    [cut]: { shape: 1, player: 1, x: shifted, y: HEADER_H + MOVE_Y, at: [0, 0.3] },
    [n]: {
      ...third[n], at: [0.3, 0.65],
      through: { x: (pot(1) + columnX(slotted.length - 2)) / 2, y: HEADER_H + MOVE_Y - ARC },
    },
    [taxN[0]]: { ...third[taxN[0]], keys: [[0.3, { x: shifted }], [0.65, { x: shifted }]] },
    // The label over the pot gives way while n passes over it.
    out: { ...third.out, keys: [[0.3, { opacity: 0 }], [0.6, { opacity: 0 }]] },
    [payId('', cut)]: { opacity: 0, at: [0.65, 1] },
    [payId('', n)]: { at: [0.65, 1] },
  });
  const fourth = { ...built([]), ...score('score-b') };
  fourth[dropped] = { ...fourth[dropped], ...potLook(dropped, n, { given: true }) };
  return { list: [first, second, third, fourth], height, width: rowWidth(REFERENCE.columns, REFERENCE.pot) };
}

/** Mount the figure in host, which holds an <svg>, .fig-timeline and .fig-caption; returns the facts and the figure's
 *  handles for the prose. */
export function mountSlip(host, { small }) {
  const f = slipOfRecord(small, N);
  const { before, after, cut, dropped } = f;
  const { list, height, width } = states(f);
  const s = scene(host.querySelector('svg'), {
    width, height, className: 'is-dense',
    label: `Game ${N}'s game built from game ${N - 1}'s optimal game: ${N} goes just before ${cut}, takes the tax `
      + `of ${cut} and leaves ${cut} out, in four states.  The text under the figure describes the current state.`,
  });
  s.svg.style.maxWidth = `${REFERENCE.rem}rem`;
  addRow(s, before, '', HEADER);
  s.line(payId('', N), N, after.turns.find((t) => t.move === N).tax[0], 'pay');
  s.text('score-a', fmt(before.score), { kind: 'score' });
  s.text('score-b', scoreWords(f), { kind: 'score' });
  s.states(list);
  const text = words(f);
  const caption = host.querySelector('.fig-caption');
  holdHeight(caption, text);
  timeline(s, host.querySelector('.fig-timeline'), {
    stops: [
      { at: 0, name: `game ${N - 1}'s game` },
      { at: 1, name: `${N} can't come last` },
      { at: 2, name: `${N} before ${cut}` },
      { at: 3, name: `leave ${dropped} out` },
    ],
    param: 'slip',
    fromParam: (value) => Number(value) - 1,
    toParam: (k) => String(k + 1),
    describe: (k) => text[k],
    title: (k) => {
      caption.textContent = text[k];
    },
    label: `Game ${N} from game ${N - 1}: state`,
  });
  return { facts: f, goState: (k) => s.go(Number(k) - 1), highlight: (x, on) => s.highlight(x, on) };
}
