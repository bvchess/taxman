// A row of a mid-sized game's frames, drawn small, to show their number: every frame of the game the page names as a
// box, a mark for each of its numbers, the sources at its left (gray if descending greedy keeps them, hollow if not)
// and its factors at its right; a dashed box is a closed frame.  The boxes stand in the wiki's order, the highest layer
// at the left and the frame in front at the right, and are named by their numbers (js/model.js, frameModel), frame 0
// being the one in front; a closed frame has no number and is named closed.  Under a box with freedoms stand its
// number and its freedoms, the sum over the frames that make it up, in one of two forms: on a line below the number
// ("2 freedoms"), or stacked, the count and then the word, when that lets the marks be larger.  Under a closed box
// stands only closed.  The labels are measured in the page, and a box is at least as wide as its widest line.
// Frames with freedoms that share a number are one frame (collapseFrames): one box, with all their sources at its
// left, largest first, and all their factors at its right.
//
// Descending greedy on a game this size takes milliseconds, so the figure computes the frames when it is mounted, and
// checks the sum of U* against the recorded in_the_bag of data/games.json.  It sits in the text column: the columns of
// a box are as tall as lets the row fit that width with the largest marks, and no box is taller than its tallest
// column.

import * as M from '../js/model.js';
import { el, fmt } from '../js/vocab.js';
import { freshSVG, tableView, tooltip } from '../js/chart.js';

const MAX_CELL = 12; // the largest mark's cell, in pixels
const MIN_ROWS = 3;
const MAX_ROWS = 60;
const PAD = 0.6; // a box's margin around its numbers, in cells
const GAP = 1; // between two boxes, in cells
const MIN_GAP = 8; // and in pixels
const LABEL_PAD = 4; // a box is wider than its widest label line by this much, in pixels
const LABEL_LINE = 15; // between the lines of a box's labels, in pixels

// A frame's name: its number, or closed.  The tooltip's version says Frame h or A closed frame.
const nameOf = (u, title = false) => {
  if (u.closed) return title ? 'A closed frame' : 'closed';
  return title ? `Frame ${u.number}` : String(u.number);
};

// The lines under a box in each form: a frame with freedoms has its number and its freedoms, on one line or stacked;
// a closed frame has only closed.
function labelsOf(u, freedoms) {
  if (u.closed) return { line: ['closed'], stacked: ['closed'] };
  const word = freedoms === 1 ? 'freedom' : 'freedoms';
  const number = String(u.number);
  return { line: [number, `${fmt(freedoms)} ${word}`], stacked: [number, fmt(freedoms), word] };
}

const sum = (frames, count) => frames.reduce((a, f) => a + count(f), 0);
// The counts of a box: the sums over the frames that make it up.
const countsOf = (frames) => ({
  sources: sum(frames, (f) => f.sources.length), kept: sum(frames, (f) => f.kept.length),
  factors: sum(frames, (f) => f.factors), freedoms: sum(frames, (f) => f.freedoms),
});

// The boxes in the order they are drawn, the highest layer first and in a layer the largest first, each with its
// sources from the largest down and its count of factors.
function boxesOf(model) {
  const frames = model.map((f) => ({ ...f, factors: f.factors.length }));
  frames.sort((a, b) => b.layer - a.layer || (b.sources.length + b.factors) - (a.sources.length + a.factors));
  return M.collapseFrames(frames).map((u) => {
    const kept = new Set(u.frames.flatMap((f) => f.kept));
    const sources = u.frames.flatMap((f) => f.sources).sort((a, b) => b - a);
    return { ...u, sources, kept, factors: sum(u.frames, (f) => f.factors) };
  });
}

// Where each box stands for columns of rows cells and cells of c pixels, and the row's width in pixels, with the
// labels in form.  A box is as wide as its columns, or as its widest label line if that is wider.
function layout(boxes, rows, c, form) {
  let x = 0;
  const placed = boxes.map((u) => {
    const sourceCols = Math.ceil(u.sources.length / rows);
    const factorCols = Math.max(1, Math.ceil(u.factors / rows));
    const w = Math.max((2 * PAD + sourceCols + 1 + factorCols) * c, u.need[form]);
    const p = {
      u, x, w, labels: u.labels[form], sourceCols, factorCols,
      rows: Math.min(rows, Math.max(u.sources.length, u.factors)),
    };
    x += w + Math.max(MIN_GAP, GAP * c);
    return p;
  });
  return { placed, width: x - Math.max(MIN_GAP, GAP * c) };
}

// The column height with the largest cell that lets the row fit width; of the heights that allow it, the lowest.
function fit(boxes, width, form) {
  let best = null;
  for (let rows = MIN_ROWS; rows <= MAX_ROWS; rows++) {
    let c = MAX_CELL;
    while (c > 1 && layout(boxes, rows, c, form).width > width) c -= 0.1;
    if (!best || c > best.c + 1e-9) best = { rows, c, form };
  }
  return best;
}

const circle = (cx, cy, r) =>
  `M${(cx - r).toFixed(2)} ${cy.toFixed(2)}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0`;
const square = (cx, cy, h) => `M${(cx - h).toFixed(2)} ${(cy - h).toFixed(2)}h${2 * h}v${2 * h}h${-2 * h}z`;

// The boxes with their labels in both forms and the width each form needs, measured in svg.
function withLabels(boxes, svg) {
  const probe = el('text', { class: 'tick' }, svg);
  const measure = (text) => {
    probe.textContent = text;
    return probe.getComputedTextLength();
  };
  const labelled = boxes.map((u) => {
    const labels = labelsOf(u, countsOf(u.frames).freedoms);
    const need = (form) => Math.max(...labels[form].map(measure)) + LABEL_PAD;
    return { ...u, labels, need: { line: need('line'), stacked: need('stacked') } };
  });
  probe.remove();
  return labelled;
}

function draw(box, tip, data, width) {
  const svg = freshSVG(box, width, 0,
    `The frames of game ${fmt(data.n)}, a mark for each number. The numbers are in the table below.`);
  svg.classList.add('frames-row-svg');
  const boxes = withLabels(data.boxes, svg);
  // The form with the larger cell, the one-line form on a tie.
  const oneLine = fit(boxes, width, 'line');
  const stacked = fit(boxes, width, 'stacked');
  const { rows, c, form } = stacked.c > oneLine.c + 1e-9 ? stacked : oneLine;
  const { placed, width: total } = layout(boxes, rows, c, form);
  const top = 6;
  const boxH = (Math.max(...placed.map((p) => p.rows)) + 2 * PAD) * c;
  const height = top + boxH + 6 + (Math.max(...placed.map((p) => p.labels.length)) * LABEL_LINE) + 6;
  const left = (width - total) / 2;
  svg.setAttribute('viewBox', `0 0 ${width} ${height}`);
  svg.setAttribute('height', height);
  let kept = '';
  let rejected = '';
  let factors = '';
  for (const p of placed) {
    const x0 = left + p.x;
    el('rect', {
      x: x0.toFixed(2), y: top, width: p.w.toFixed(2), height: boxH.toFixed(2), rx: Math.min(6, 1.4 * c),
      class: p.u.closed ? 'row-frame is-closed' : 'row-frame',
    }, svg);
    p.u.sources.forEach((s, i) => {
      const cx = x0 + (PAD + Math.floor(i / p.rows) + 0.5) * c;
      const cy = top + (PAD + (i % p.rows) + 0.5) * c;
      if (p.u.kept.has(s)) kept += circle(cx, cy, 0.4 * c);
      else rejected += circle(cx, cy, 0.32 * c);
    });
    const factorsX = x0 + p.w - (PAD + p.factorCols) * c;
    for (let i = 0; i < p.u.factors; i++) {
      const cx = factorsX + (Math.floor(i / p.rows) + 0.5) * c;
      const cy = top + (PAD + (i % p.rows) + 0.5) * c;
      factors += square(cx, cy, 0.3 * c);
    }
    p.labels.forEach((label, k) => {
      const t = el('text', {
        x: (x0 + p.w / 2).toFixed(1), y: top + boxH + 3 + (k + 1) * LABEL_LINE, 'text-anchor': 'middle',
        class: 'tick',
      }, svg);
      t.textContent = label;
    });
  }
  el('path', { d: factors, class: 'row-factor' }, svg);
  el('path', { d: kept, class: 'row-kept' }, svg);
  el('path', { d: rejected, class: 'row-rejected' }, svg);
  // The hover layer: a box's counts.
  for (const p of placed) {
    const x0 = left + p.x;
    const hit = el('rect', {
      x: (x0 - 1).toFixed(2), y: 0, width: (p.w + 2).toFixed(2), height: top + boxH + 4, fill: 'transparent',
      class: 'hit',
    }, svg);
    hit.addEventListener('pointerenter', () => {
      const counts = countsOf(p.u.frames);
      const lines = [nameOf(p.u, true)];
      lines.push(['sources', fmt(counts.sources)], ['kept by greedy', fmt(counts.kept)],
        ['factors', fmt(counts.factors)], ['freedoms', p.u.closed ? 'none: closed' : fmt(counts.freedoms)]);
      tip.show(lines, x0 + p.w / 2, top + 20);
    });
    hit.addEventListener('pointerleave', () => tip.hide());
  }
}

/** Mount the figure in host, which holds .row-box and .fig-caption.  Returns its facts (M.frameFacts, with n and
 *  candidates), which the page's prose quotes.  Throws if descending greedy does not give the recorded sum of U*. */
export function mountFramesRow(host, { games, n }) {
  const box = host.querySelector('.row-box');
  const caption = host.querySelector('.fig-caption');
  const i = games.columns.n.indexOf(n);
  const model = M.frameModel(n);
  const inTheBag = sum(model, (f) => sum(f.kept, (s) => s));
  if (inTheBag !== games.columns.inTheBag[i]) {
    caption.textContent = `Game ${fmt(n)}: descending greedy does not give the recorded sum of U*.`;
    throw new Error(`game ${n}: U* sums to ${inTheBag}, and the results file records ${games.columns.inTheBag[i]}`);
  }
  const data = { n, boxes: boxesOf(model) };
  const facts = { ...M.frameFacts(model), n, candidates: games.columns.candidates[i] };
  const numbers = sum(model, (f) => f.sources.length + f.factors.length);
  caption.textContent = `The frames of game ${fmt(n)}, a mark for each of its ${fmt(numbers)} numbers: in each frame ` +
    'its sources at the left, gray where descending greedy keeps them and hollow where it does not, and its factors ' +
    `at the right. Under each frame with freedoms are its number and its freedoms, and the ${facts.closed} closed ` +
    'frames are dashed.';
  const columns = ['frame', 'sources', 'kept', 'factors', 'freedoms'];
  tableView(host, 'The numbers behind the figure', columns, data.boxes.map((u) => {
    const counts = countsOf(u.frames);
    return [nameOf(u), fmt(counts.sources), fmt(counts.kept), fmt(counts.factors),
      u.closed ? 'closed' : fmt(counts.freedoms)];
  }));
  const tip = tooltip(box);
  let width = 0;
  new ResizeObserver(() => {
    const w = Math.round(box.clientWidth);
    if (w > 0 && w !== width) {
      width = w;
      draw(box, tip, data, width);
    }
  }).observe(box);
  return facts;
}
