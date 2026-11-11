// The explorer of chapter 8: any game of the chain, with what the chain records for it, and, for the games of
// data/small.json, the optimal game itself on its board.
//
// The record is data/games.json's.  The game is data/small.json's, replayed with js/model.js, which gives each move's
// tax and the rest.  The board is flat, since it explains: ten numbers a row, the player's moves blue circles, the
// taxman's numbers orange, a square up to n/2 and a circle above it, and a dashed circle a number above n/2 that the
// game does not play.
//
// The explorer fills the center column: the controls, then the record, then the board under it.  Every number keeps
// its place on the board from game to game, and the board has the rows of the game shown.  Each line of the record
// keeps the room of its longest form at the present width, measured when the width changes, never when n does.  The
// caption stands above the board and keeps the room of the longest of the game's captions, measured when the game or
// the width changes.  A game past those of small.json has no board: its board area is not shown.
//
// URL parameters: n=N opens game N, and at=x, with a game up to 100, opens it pointing at the number x.

import * as M from '../js/model.js';
import { NUMBER, duration, fmt, listWords, numberMark, setNumber } from '../js/vocab.js';
import { PROOFS, PROVER_TO, foundationOf } from '../js/proof.js';

// The game the explorer opens at, when the address names none.
const FIRST_GAME = 21;

const COLS = 10;
const CELL = 40;
const ROWS_OF = (n) => Math.ceil(n / COLS);

const sum = (xs) => xs.reduce((a, b) => a + b, 0);
const total = (n) => (n * (n + 1)) / 2;
const plural = (k, one, many = `${one}s`) => `${fmt(k)} ${k === 1 ? one : many}`;
const percent = (x) => `${(100 * x).toFixed(2)}%`;
const upper = (s) => s.charAt(0).toUpperCase() + s.slice(1);

/** The largest prime below n, or null below 3. */
function primeBelow(n) {
  for (let q = n - 1; q >= 2; q--) if (M.primeFactors(q).length === 1 && M.primeFactors(q)[0] === q) return q;
  return null;
}

/** What the chain records for game n, read from the columns of games.json, with the words the explorer shows. */
export function record(games, n) {
  const c = games.columns;
  const i = n - 1;
  if (c.n[i] !== n) throw new Error(`games.json: row ${i} is game ${c.n[i]}, not game ${n}`);
  const r = {
    n, score: c.score[i], proof: c.proof[i], method: c.method[i], nodes: c.nodes[i], seconds: c.seconds[i],
    moves: c.moves[i], promotions: c.promotions[i], candidates: c.candidates[i], inTheBag: c.inTheBag[i],
  };
  r.board = total(n);
  r.taxman = r.board - r.score;
  r.prev = n > 1 ? c.score[i - 1] : null;
  r.settled = settled(r);
  r.rests = restsOn(r, foundationOf(c, i));
  return r;
}

// How the game was settled: the proof kind's name, as the chain's chart gives it, and what it did, with its numbers.
function settled(r) {
  const kind = PROOFS.find((p) => p.key === r.proof);
  if (!kind) throw new Error(`game ${r.n}: no proof kind ${r.proof}`);
  const lead = `${upper(kind.name)}.`;
  const { n, score, prev } = r;
  switch (r.proof) {
    case 'exhausted':
      return [lead, `The search explored ${plural(r.nodes, 'node')}, and no better set of promotions remains.`];
    case 'cap': {
      if (score !== prev + n) throw new Error(`game ${n}: proof cap, but ${score} is not ${prev} + ${n}`);
      const how = r.method === 'start'
        ? 'The search’s starting game reached it, so the search took no step.'
        : `The search reached it after ${plural(r.nodes, 'node')}.`;
      return [lead, `Its score is opt(${fmt(n - 1)}) + ${fmt(n)} = ${fmt(prev)} + ${fmt(n)} = ${fmt(score)}, the most a game can make in game ${fmt(n)}. ${how}`];
    }
    case 'prime_formula': {
      const q = primeBelow(n);
      if (q === null) {
        if (score !== prev + n) throw new Error(`game ${n}: the prime formula does not give ${score}`);
        return [lead, `Since ${n} is the first prime, its score is opt(${n - 1}) + ${n} = ${fmt(score)}.`];
      }
      if (score !== prev + n - q) throw new Error(`game ${n}: the prime formula gives ${prev + n - q}, not ${score}`);
      return [lead, `Since ${fmt(n)} is prime, its score is opt(${fmt(n - 1)}) + ${fmt(n)} − ${fmt(q)} = ${fmt(score)}, where ${fmt(q)} is the largest prime below ${fmt(n)}.`];
    }
    case 'same_search':
      return [lead, `Game ${fmt(n)} poses the same search problem as an earlier game, whose promotions carry over with no search.`];
    case 'trivial':
      return [lead, n === 1
        ? 'Game 1 has no move: the taxman takes the 1.'
        : 'The player can promote no factor, so the game that promotes nothing is the only one.'];
    default:
      throw new Error(`game ${n}: no words for proof ${r.proof}`);
  }
}

// What the score depends on.
function restsOn({ n }, f) {
  if (n === 1) return 'Nothing: game 1 has no move to choose.';
  if (n <= PROVER_TO) return `The prover proves it too, on the whole board, so it is opt(${fmt(n)}) without the frame model.`;
  if (f <= PROVER_TO) {
    return `It follows from game ${fmt(f)}’s score, which the prover proves on the whole board, so it is opt(${fmt(n)}) without the frame model.`;
  }
  if (f < n) {
    return `It follows from game ${fmt(f)}’s score, the frame model’s best, which is opt(${fmt(f)}) when statement U holds, and U is open past game ${fmt(PROVER_TO)}.`;
  }
  return `It is the frame model’s best score, which is opt(${fmt(n)}) when statement U holds, and U is open past game ${fmt(PROVER_TO)}.`;
}

// The six numbers under the words: each a label, a value and a line under it.
function stats(r) {
  const noFrames = 'the prime formula sets up no frames';
  const bag = r.inTheBag;
  return [
    ['moves', fmt(r.moves), r.n - r.moves === 1 ? '1 number goes to the taxman' : `${fmt(r.n - r.moves)} numbers go to the taxman`],
    ['promotions', fmt(r.promotions), bag === null ? '' : `worth ${fmt(r.score - bag)}`],
    ['the top half, ΣU*', bag === null ? '—' : fmt(bag), bag === null ? noFrames : r.score > 0 ? `${percent(bag / r.score)} of the score` : ''],
    ['candidates', r.candidates === null ? (r.proof === 'trivial' ? 'none' : '—') : fmt(r.candidates), r.candidates === null && r.proof !== 'trivial' ? noFrames : ''],
    ['nodes', fmt(r.nodes), r.nodes === 0 ? 'no step of search' : ''],
    ['time', duration(r.seconds), 'with at least one thread at work'],
  ];
}

// ---------------------------------------------------------------------------------------------------------------
// The board
// ---------------------------------------------------------------------------------------------------------------

/** Who takes each number of game n in its recorded game, and when: a Map from x to { kind: 'move' | 'tax' | 'rest',
 *  turn, tax, by }, with the moves in order and the score, checked against the record. */
export function boardOf(game) {
  const { n } = game;
  const played = M.replay(n, game.moves);
  if (M.score(played) !== game.score) throw new Error(`game ${n}: the recorded moves score ${M.score(played)}, not ${game.score}`);
  const who = new Map();
  const turns = played.log.filter((e) => e.move !== undefined);
  turns.forEach((e, i) => {
    who.set(e.move, { kind: 'move', turn: i + 1, tax: e.tax });
    for (const d of e.tax) who.set(d, { kind: 'tax', turn: i + 1, by: e.move });
  });
  const rest = played.log.find((e) => e.rest)?.rest ?? [];
  for (const d of rest) who.set(d, { kind: 'rest' });
  return { who, turns, rest, promotions: game.promotions, taxman: sum(played.taxman) };
}

const lookOf = (n, x, w, halo = 0) => {
  if (w.kind === 'move') return { ...NUMBER, shape: 1, player: 1, halo };
  if (x > n / 2) return { ...NUMBER, shape: 1, taxman: 1, dashed: 1, halo };
  return { ...NUMBER, shape: 0, taxman: 1, halo };
};

const at = (x) => ({ x: CELL / 2 + CELL * ((x - 1) % COLS), y: CELL / 2 + CELL * Math.floor((x - 1) / COLS) });

// ---------------------------------------------------------------------------------------------------------------
// Mounting
// ---------------------------------------------------------------------------------------------------------------

/** Mount the explorer in host, an empty figure.  Returns { set(n), get n }. */
export function mountExplorer(host, { games, small }) {
  const last = games.lastN;
  const smallGames = new Map(small.map((g) => [g.n, g]));
  const W = CELL * COLS;

  host.innerHTML = `
    <div class="ex-controls">
      <div class="ex-choose">
        <span class="stepper" role="group" aria-label="Choose a game">
          <button type="button" class="btn round" data-act="down" aria-label="The game before">&minus;</button>
          <label class="ex-n-label"><span>Game</span><input class="ex-n" type="number" min="1" max="${last}" step="1" inputmode="numeric"></label>
          <button type="button" class="btn round" data-act="up" aria-label="The next game">+</button>
        </span>
        <input class="ex-range" type="range" min="1" max="${last}" step="1" aria-label="Game n, from game 1 to game ${fmt(last)}">
      </div>
    </div>
    <div class="ex-body">
      <div class="ex-record">
        <div class="ex-totals">
          <div class="ex-total is-player"><span class="ex-who">The player</span><span class="ex-big" data-f="score"></span></div>
          <div class="ex-total is-taxman"><span class="ex-who">The taxman</span><span class="ex-big" data-f="taxman"></span></div>
        </div>
        <p class="ex-share" data-f="share" aria-live="polite"></p>
        <dl class="ex-words">
          <dt>How the chain settled it</dt><dd data-f="settled"></dd>
          <dt>What it depends on</dt><dd data-f="rests"></dd>
        </dl>
        <dl class="ex-stats"></dl>
      </div>
      <div class="ex-board">
        <p class="ex-caption"></p>
        <svg class="v-figure is-flat ex-svg" viewBox="0 0 ${W} ${CELL}" role="img" tabindex="0"></svg>
        <p class="ex-caption ex-ruler"></p>
      </div>
    </div>`;

  const $ = (sel) => host.querySelector(sel);
  const input = $('.ex-n');
  const range = $('.ex-range');
  const svg = $('.ex-svg');
  const caption = $('.ex-caption');
  const ruler = $('.ex-ruler');
  const board$ = $('.ex-board');
  const field = (name) => host.querySelector(`[data-f="${name}"]`);
  const statsBox = $('.ex-stats');
  statsBox.innerHTML = stats(record(games, 1)).map(() => '<div class="ex-stat"><dt></dt><dd><span class="ex-val"></span><span class="ex-sub"></span></dd></div>').join('');
  const statCells = [...statsBox.querySelectorAll('.ex-stat')];

  let n = 0;
  let board = null; // the board of the game drawn, when it is drawn
  let marks = new Map();
  let lit = [];
  let focusTurn = 0;
  let held = null; // a number the address points at, kept through a redraw at a new width

  // ---- words ------------------------------------------------------------------------------------------------

  const settledHTML = ([lead, rest]) => `<b>${lead}</b> ${rest}`;
  const shareText = (r) => `The player’s ${fmt(r.score)}, opt(${fmt(r.n)}), is ${percent(r.score / r.board)} of the board’s total, ${r.n <= 3 ? Array.from({ length: r.n }, (_, k) => k + 1).join(' + ') : `1 + 2 + ⋯ + ${fmt(r.n)}`} = ${fmt(r.board)}.`;

  function defaultCaption(g, b) {
    const p = b.promotions;
    const promoted = p.length === 0
      ? 'It promotes nothing.'
      : `${listWords(p)} ${p.length === 1 ? 'is a move up to n/2, a promotion' : 'are moves up to n/2, its promotions'}.`;
    return `The player’s ${plural(b.turns.length, 'move')}, in blue, score ${fmt(g.score)}; the taxman’s ${plural(g.n - b.turns.length, 'number')}, in orange, total ${fmt(b.taxman)}. ${promoted} Point at a number to see where it goes.`;
  }

  function aboutNumber(g, b, x) {
    const w = b.who.get(x);
    if (w.kind === 'move') {
      const promo = b.promotions.includes(x) ? ` It is up to n/2, so it is a promotion.` : '';
      return `Move ${w.turn} of ${b.turns.length}: the player takes ${x}, and the taxman takes ${listWords(w.tax)}.${promo}`;
    }
    if (w.kind === 'tax') return `${x} goes to the taxman as tax on ${w.by}, move ${w.turn} of ${b.turns.length}.`;
    return `${x} remains when no move does, so the taxman takes it with the rest of the pot, ${plural(b.rest.length, 'number')} in all.`;
  }

  // ---- the room each line of words needs ---------------------------------------------------------------------

  // The longest few strings of each kind, by length, stand for all: each is set in its box and measured, and the box
  // keeps the tallest height.  Done when the width changes.
  const longest = (strings, k = 4) => [...new Set(strings)].sort((a, b) => b.length - a.length).slice(0, k);
  const samples = (() => {
    const all = Array.from({ length: last }, (_, i) => record(games, i + 1));
    const allStats = all.map(stats);
    const subs = statCells.map((_, k) => longest(allStats.map((s) => s[k][2]), 2));
    return {
      share: longest(all.map(shareText)),
      settled: longest(all.map((r) => settledHTML(r.settled))),
      rests: longest(all.map((r) => r.rests)),
      subs,
    };
  })();

  function reserve() {
    const fit = (node, strings, html = false) => {
      node.style.minHeight = '';
      let h = 0;
      for (const s of strings) {
        if (html) node.innerHTML = s;
        else node.textContent = s;
        h = Math.max(h, node.getBoundingClientRect().height);
      }
      node.style.minHeight = `${Math.ceil(h)}px`;
    };
    fit(field('share'), samples.share);
    fit(field('settled'), samples.settled, true);
    fit(field('rests'), samples.rests);
    statCells.forEach((cell, k) => fit(cell.querySelector('.ex-sub'), samples.subs[k].filter(Boolean).length ? samples.subs[k] : ['\u00a0']));
    render(true);
    if (held) point(held);
  }

  // The caption keeps the room of the tallest of the game's captions, its own and the one for each number, so that
  // pointing along the board never moves the board.  Each is set in a hidden copy of the caption and measured, when
  // the board is drawn and when the width changes.
  function fitCaption() {
    const g = smallGames.get(n);
    let h = 0;
    for (const s of [defaultCaption(g, board), ...Array.from({ length: n }, (_, i) => aboutNumber(g, board, i + 1))]) {
      ruler.textContent = s;
      h = Math.max(h, ruler.getBoundingClientRect().height);
    }
    ruler.textContent = '';
    caption.style.minHeight = `${Math.ceil(h)}px`;
  }

  // ---- drawing ----------------------------------------------------------------------------------------------

  function drawBoard() {
    svg.replaceChildren();
    marks = new Map();
    lit = [];
    focusTurn = 0;
    const g = smallGames.get(n);
    if (!g) {
      board = null;
      return;
    }
    if (g.score !== record(games, n).score) throw new Error(`game ${n}: small.json and games.json differ`);
    board = boardOf(g);
    svg.setAttribute('viewBox', `0 0 ${W} ${CELL * ROWS_OF(n)}`);
    svg.tabIndex = 0;
    const moves = board.turns.map((t) => t.move);
    svg.setAttribute('aria-label', `The board of game ${n}, ten numbers a row. The player takes ${listWords(moves)}, in that order, and the taxman takes the rest. Use the arrow keys to step through the moves.`);
    for (let x = 1; x <= n; x++) {
      const mark = numberMark(svg, x);
      mark.body.style.cursor = 'pointer';
      marks.set(x, mark);
      setNumber(mark, { ...lookOf(n, x, board.who.get(x)), ...at(x) });
    }
  }

  function light(xs) {
    for (const x of lit) if (marks.has(x)) setNumber(marks.get(x), { ...lookOf(n, x, board.who.get(x)), ...at(x) });
    lit = xs.filter((x) => marks.has(x));
    for (const x of lit) setNumber(marks.get(x), { ...lookOf(n, x, board.who.get(x), 1), ...at(x) });
  }

  function point(x) {
    if (!board) return;
    const g = smallGames.get(n);
    if (x === null) {
      light([]);
      caption.textContent = defaultCaption(g, board);
      return;
    }
    const w = board.who.get(x);
    if (w.kind === 'move') light([x, ...w.tax]);
    else if (w.kind === 'tax') light([x, w.by]);
    else light(board.rest);
    caption.textContent = aboutNumber(g, board, x);
  }

  function render(widthOnly = false) {
    const r = record(games, n);
    if (!widthOnly) {
      if (document.activeElement !== input || Number(input.value) !== n) input.value = n;
      range.value = n;
      range.style.setProperty('--progress', `${(100 * (n - 1)) / Math.max(1, last - 1)}%`);
      host.querySelector('[data-act="down"]').disabled = n <= 1;
      host.querySelector('[data-act="up"]').disabled = n >= last;
      drawBoard();
    }
    field('score').textContent = fmt(r.score);
    field('taxman').textContent = fmt(r.taxman);
    field('share').textContent = shareText(r);
    field('settled').innerHTML = settledHTML(r.settled);
    field('rests').textContent = r.rests;
    stats(r).forEach(([label, value, sub], k) => {
      const cell = statCells[k];
      cell.querySelector('dt').textContent = label;
      cell.querySelector('.ex-val').textContent = value;
      cell.querySelector('.ex-sub').textContent = sub;
    });
    board$.hidden = !board;
    if (board) {
      fitCaption();
      point(null);
    }
  }

  // The address keeps the game chosen, a little after the reader stops, so that a slider dragged across the chain
  // does not rewrite it at every step.
  let addressTimer = 0;
  function keepAddress() {
    clearTimeout(addressTimer);
    addressTimer = setTimeout(() => {
      const url = new URL(location.href);
      url.searchParams.set('n', String(n));
      history.replaceState(history.state, '', url);
    }, 400);
  }

  function set(value, { address = true } = {}) {
    const next = Math.min(last, Math.max(1, Math.round(Number(value))));
    if (!Number.isFinite(next) || next === n) return;
    n = next;
    held = null;
    render();
    if (address) keepAddress();
  }

  // ---- the reader's hand ------------------------------------------------------------------------------------

  input.addEventListener('input', () => {
    const v = Number(input.value);
    if (input.value !== '' && Number.isInteger(v) && v >= 1 && v <= last) set(v);
  });
  input.addEventListener('change', () => { input.value = n; });
  range.addEventListener('input', () => set(range.value));
  host.querySelector('[data-act="down"]').addEventListener('click', () => set(n - 1));
  host.querySelector('[data-act="up"]').addEventListener('click', () => set(n + 1));

  const numberAt = (e) => {
    const g = e.target.closest?.('.v-num');
    return g ? Number(g.dataset.n) : null;
  };
  svg.addEventListener('pointerover', (e) => {
    const x = numberAt(e);
    if (x !== null) point(x);
  });
  svg.addEventListener('pointerleave', () => {
    held = null;
    point(null);
  });
  svg.addEventListener('click', (e) => point(numberAt(e)));
  svg.addEventListener('keydown', (e) => {
    if (!board) return;
    const count = board.turns.length;
    const to = { ArrowRight: focusTurn + 1, ArrowDown: focusTurn + 1, ArrowLeft: focusTurn - 1, ArrowUp: focusTurn - 1, Home: 1, End: count }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    focusTurn = Math.min(count, Math.max(1, to));
    point(board.turns[focusTurn - 1].move);
  });
  svg.addEventListener('blur', () => {
    focusTurn = 0;
    point(null);
  });

  const params = new URL(location.href).searchParams;
  const asked = Number(params.get('n'));
  set(Number.isInteger(asked) && asked >= 1 && asked <= last ? asked : FIRST_GAME, { address: false });
  const pointed = Number(params.get('at'));
  held = board && marks.has(pointed) ? pointed : null;
  if (held) point(held);

  let lastWidth = 0;
  new ResizeObserver(() => {
    const w = Math.round(host.getBoundingClientRect().width);
    if (w > 0 && w !== lastWidth) {
      lastWidth = w;
      reserve();
    }
  }).observe(host);
  document.fonts?.ready.then(() => {
    lastWidth = 0;
    reserve();
  });

  return { set, get n() { return n; } };
}
