// The idea in three steps, each with a small still drawing in the flat rendering, in the layout of the game-21
// figure: a set of moves in two orders, one frame with its sources at the left and its factors at the right, and a
// promotion, a short hop rightward into the frame in front.  Game 21's numbers, from js/model.js.

import * as M from '../js/model.js';
import { SHAPE } from '../js/vocab.js';
import { scene } from '../js/scene.js';

const DRAW = {
  // The same three moves in two orders: the same set, so the same score.
  orders(s) {
    const rows = [[19, 9, 15], [19, 15, 9]];
    const look = {};
    rows.forEach((list, r) => list.forEach((x, i) => {
      s.number(`${r}-${x}`, x);
      look[`${r}-${x}`] = { x: 30 + i * 40, y: 30 + r * 46, shape: SHAPE.circle, kept: 1 };
    }));
    s.text('same', 'same set', { anchor: 'start' });
    look.same = { x: 134, y: 53 };
    return look;
  },
  // Game 21's frame 0, as the wiki lays a frame out.
  frame(s) {
    const f = M.frameModel(21).find((x) => x.number === 0);
    const look = { box: { x: 30, y: 4, w: 140, h: 108 } };
    s.box('box');
    // Each column centered on the frame's middle, the sources a little further apart than the factors.
    const y = (i, m, step) => 58 + step * (i - (m - 1) / 2);
    f.sources.forEach((x, i) => {
      s.number(`s${x}`, x);
      look[`s${x}`] = { x: 66, y: y(i, f.sources.length, 30), shape: SHAPE.circle, kept: 1, scale: 0.82 };
    });
    f.factors.forEach((x, i) => {
      s.number(`f${x}`, x);
      look[`f${x}`] = { x: 134, y: y(i, f.factors.length, 25), shape: SHAPE.square, scale: 0.82 };
    });
    for (const x of f.sources) for (const p of M.maximalFactors(x)) {
      s.line(`${x}-${p}`, `s${x}`, `f${p}`, 'thin');
      look[`${x}-${p}`] = {};
    }
    return look;
  },
  // A promotion: 9 leaves the factors of one frame, rounds into a move of the frame in front, and 3 pays for it.
  promote(s) {
    s.box('from').box('into');
    s.number('ghost', 9).number('nine', 9).number('three', 3);
    s.line('rise', 'ghost', 'nine', 'hop').line('pay', 'nine', 'three', 'pay');
    return {
      from: { x: 14, y: 22, w: 46, h: 76 },
      into: { x: 78, y: 22, w: 108, h: 76 },
      ghost: { x: 37, y: 60, shape: SHAPE.square, dashed: 1, faded: 0.7 },
      nine: { x: 108, y: 60, shape: SHAPE.circle, kept: 1, ring: 1 },
      three: { x: 160, y: 60, shape: SHAPE.square },
      rise: {},
      pay: {},
    };
  },
};

const LABELS = {
  orders: 'The moves 19, 9 and 15, and the same three in another order: the same set.',
  frame: 'Game 21’s frame 0: the sources 14, 15 and 21 at the left, the factors 2, 3, 5 and 7 at the right.',
  promote: 'A promotion: the factor 9 leaves its frame, becomes a move of the frame in front, and 3 pays for it.',
};

export function mountSteps(host) {
  const kinds = ['orders', 'frame', 'promote'];
  host.querySelectorAll('.step-pic-host').forEach((h, i) => {
    const s = scene(h, { width: 200, height: 116, label: LABELS[kinds[i]], className: 'step-pic' });
    s.states([DRAW[kinds[i]](s)]).render(0);
  });
}
