// Does n fit?  Two panels of game rows, one for a game n that fits and one for a game n that does not: each shows the
// optimal game of game n − 1, above, and a game of n, below, in the same columns, each move over its tax and at the
// right what is left in the pot (figures/game-row.js).  When n fits, the game below is the one above with n played last,
// paid for by a number the game above left in the pot.  When it does not, it is game n's optimal game, which gives up a
// move of the game above so that the move can pay for n.  A halo marks what n uses and what changes.  A flat figure.
//
// The games are data/small.json's, and the panels' words are worked out from them: whether n fits is decided by
// peel (js/model.js, isPlayable), the maximal factors of n by js/model.js, the taxes and the scores by replaying the
// moves.  The figure checks that game n scores its ceiling, opt(n − 1) + n, exactly when n fits.

import * as M from '../js/model.js';
import { scene } from '../js/scene.js';
import { fmt, listWords } from '../js/vocab.js';
import { slipFacts } from './slip.js';
import {
  MOVE_Y, PAD, addRow, gameRow, rowHeight, rowLooks, rowWidth,
} from './game-row.js';

const LABEL = 82; // the width of a row's name
const ascending = (a, b) => a - b;
const sum = (list) => list.reduce((total, x) => total + x, 0);

/** The facts of the panel of n, from the recorded games: { n, fits, above, below, uses, gaveUp, slots }.  above is
 *  game n − 1's optimal game, below a game of n (the one above with n played in when n fits, game n's own when not),
 *  uses the numbers of the game above that pay for n, gaveUp the moves the game below gives up, and slots the move of
 *  each column.  Throws if game n's score is not opt(n − 1) + n exactly when n fits. */
export function fitsFacts(small, n) {
  const prev = small[n - 2];
  const cur = small[n - 1];
  const fail = (why) => {
    throw new Error(`fits of ${n}: ${why}`);
  };
  if (prev?.n !== n - 1 || cur?.n !== n) fail('small.json has no games n − 1 and n');
  const fits = M.isPlayable([...prev.moves, n]);
  if (fits !== (cur.score === prev.score + n)) fail(`n ${fits ? 'fits' : 'does not fit'}, but game ${n} scores ${cur.score}`);
  const above = gameRow(n - 1, prev.moves);
  if (above.score !== prev.score) fail(`game ${n - 1}'s record scores ${prev.score}`);
  let below;
  if (fits) {
    const slip = slipFacts(n, prev.moves);
    if (slip.dropped !== null || slip.after.moves.join() !== [...prev.moves, n].join()) fail('n does not go last');
    below = slip.after;
  } else below = gameRow(n, cur.moves);
  if (below.score !== cur.score) fail(`the game below scores ${below.score}, not ${cur.score}`);
  const tax = below.turns.find((t) => t.move === n).tax;
  const gaveUp = above.moves.filter((m) => !below.moves.includes(m));
  const uses = fits ? tax : M.maximalFactors(n).sort(ascending);
  if (!fits && (below.score !== prev.score + n - sum(gaveUp))) fail('the game below does not give up what it loses');
  // The columns of the game above, then the moves only the game below has.
  const slots = [...above.moves, ...below.moves.filter((m) => !above.moves.includes(m))];
  return { n, fits, prev, cur, above, below, tax, uses, gaveUp, slots };
}

// How each number that can pay for n stands in the game above: a move, tax of a move, or left in the pot.
function standing(above, x) {
  const payer = above.turns.find((t) => t.tax.includes(x));
  if (payer) return `${x} pays for ${payer.move}`;
  return above.moves.includes(x) ? `${x} is a move` : `${x} is left in the pot`;
}

/** The panel's words as HTML: the lead in bold, then why. */
export function fitsWords(f) {
  const { n, above, tax, uses, gaveUp, fits, prev, cur } = f;
  if (fits) {
    return `<strong>${n} fits.</strong> Game ${n - 1}'s optimal game leaves ${listWords(tax)} in the pot, and ${n} `
      + `follows it, paid for by ${listWords(tax)}: ${fmt(prev.score)} + ${n} = ${fmt(cur.score)}, the ceiling.`;
  }
  return `<strong>${n} does not fit.</strong> Only ${listWords(uses)}, its maximal factors, can pay for it, and in game `
    + `${n - 1}'s optimal game ${uses.map((x) => standing(above, x)).join(' and ')}.  Game ${n}'s optimal game gives up `
    + `${listWords(gaveUp)}, which pays for ${n}: ${fmt(prev.score)} + ${n} − ${gaveUp.join(' − ')} = ${fmt(cur.score)}.`;
}

const rowName = (row) => `Game ${row.n}`;
const plain = (html) => html.replace(/<[^>]+>/g, '');

/** Draw the panels of fig: each .arrival in it with a data-board n, its svg and its .arrival-says.  Returns the facts
 *  of each panel and a highlight for the prose. */
export function mountFits(fig, { small }) {
  const hosts = [...fig.querySelectorAll('.arrival')];
  const panels = hosts.map((host) => ({ host, f: fitsFacts(small, Number(host.dataset.board)) }));
  // Every panel is drawn at one scale: the widest sets the width.
  const columns = Math.max(...panels.map(({ f }) => f.slots.length));
  const pot = Math.max(...panels.flatMap(({ f }) => [f.above.rest.length, f.below.rest.length]));
  const rows = panels.flatMap(({ f }) => [f.above, f.below]);
  const rowTop = rowHeight(rows) + 10;
  const height = rowTop + rowHeight(rows) - 10 + 6;
  const scenes = panels.map(({ host, f }) => {
    const words = fitsWords(f);
    const s = scene(host.querySelector('svg'), {
      width: rowWidth(columns, pot, LABEL), height, className: 'is-dense',
      label: `${rowName(f.above)} and game ${f.n}, each move over the tax it takes. ${plain(words)}`,
    });
    const left = PAD + LABEL;
    const halo = { a: new Set(f.uses), b: new Set([f.n, ...f.tax]) };
    const looks = {};
    [['a', f.above, 0], ['b', f.below, rowTop]].forEach(([tag, row, top]) => {
      addRow(s, row, tag);
      s.text(`${tag}name`, rowName(row), { kind: 'name' });
      Object.assign(looks, rowLooks(row, { slots: f.slots, left, top, tag, halo: halo[tag] }));
      looks[`${tag}name`] = { x: PAD, y: top + MOVE_Y };
    });
    s.states([looks]).show(0);
    const says = host.querySelector('.arrival-says');
    if (says) says.innerHTML = words;
    return s;
  });
  return {
    facts: panels.map(({ f }) => f),
    highlight: (x, on) => scenes.forEach((s) => ['a', 'b'].forEach((tag) => s.highlight(`${tag}${x}`, on))),
  };
}
