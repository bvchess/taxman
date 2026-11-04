// A row of a large game's frames, drawn small, to show the scale: every frame of the last game of the chain as a box,
// a mark for each of its numbers, the sources at its left (gray if descending greedy keeps them, hollow if not) and
// its factors at its right; a dashed box is a closed frame.  The frames stand in the wiki's order, the highest layer
// at the left and the frame in front at the right.
//
// The frames come from js/model.js at once (frames), and what descending greedy keeps a moment later, from a worker
// (figures/frames-row-worker.js, frameModel), since descending greedy on a board of thousands takes seconds.  The
// figure is laid out from the frames alone, so it keeps its size when the worker's answer fills it in, and it checks
// the sum of U* against the recorded in_the_bag of data/games.json.  Every frame's box holds its numbers in columns of
// the same height, chosen so that the row fits the width it has as a band, not a tower.

import * as M from '../js/model.js';
import { el, fmt, listWords } from '../js/vocab.js';
import { freshSVG, tableView, tooltip } from '../js/chart.js';

const ROWS = [24, 28, 32, 36, 40, 48, 56, 64, 72, 80, 96, 112, 128];
const MAX_CELL = 7;
const PAD = 0.6; // a box's margin around its numbers, in cells
const GAP = 1; // between two frames of one layer, in cells
const LAYER_GAP = 2.5; // between layers
// The height the row aims at, in pixels, for a width: a band across the page, never a tower.
const targetHeight = (width) => Math.min(230, Math.max(160, 0.3 * width));

// The frames in the order they are drawn: the highest layer first, and in a layer the largest first.
const drawOrder = (frames) => [...frames].sort((a, b) => b.layer - a.layer || (b.sources.length + b.factors) - (a.sources.length + a.factors));

// The columns of each frame for a column height of rows, and the row's width in cells.
function layout(frames, rows) {
  let x = 0;
  const boxes = frames.map((f, i) => {
    if (i > 0) x += frames[i - 1].layer === f.layer ? GAP : LAYER_GAP;
    const sourceCols = Math.ceil(f.sources.length / rows);
    const factorCols = Math.max(1, Math.ceil(f.factors / rows));
    const w = PAD + sourceCols + 1 + factorCols + PAD;
    const box = { f, x, w, sourceCols, factorCols };
    x += w;
    return box;
  });
  return { boxes, width: x };
}

const circle = (cx, cy, r) => `M${(cx - r).toFixed(2)} ${cy.toFixed(2)}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0`;
const square = (cx, cy, h) => `M${(cx - h).toFixed(2)} ${(cy - h).toFixed(2)}h${2 * h}v${2 * h}h${-2 * h}z`;

function draw(box, tip, data, width) {
  const frames = drawOrder(data.frames);
  // The column height that brings the row nearest the height it aims at, at the width it has.
  const heightAt = (r) => (r + 2 * PAD) * Math.min(MAX_CELL, width / layout(frames, r).width);
  const rows = ROWS.reduce((best, r) => (Math.abs(heightAt(r) - targetHeight(width)) < Math.abs(heightAt(best) - targetHeight(width)) ? r : best));
  const { boxes, width: cells } = layout(frames, rows);
  const c = Math.min(MAX_CELL, width / cells);
  const top = 6;
  const boxH = (rows + 2 * PAD) * c;
  const height = top + boxH + 30;
  const left = (width - cells * c) / 2;
  const svg = freshSVG(box, width, height, `The ${data.frames.length} frames of game ${fmt(data.n)}, a mark for each number. The numbers are in the table below.`);
  svg.classList.add('frames-row-svg');
  let kept = '';
  let rejected = '';
  let factors = '';
  for (const b of boxes) {
    const x0 = left + b.x * c;
    el('rect', {
      x: x0.toFixed(2), y: top, width: (b.w * c).toFixed(2), height: boxH.toFixed(2), rx: Math.min(6, 1.4 * c),
      class: b.f.closed ? 'row-frame is-closed' : 'row-frame',
    }, svg);
    const keptSet = new Set(b.f.kept ?? []);
    // The sources from the largest down, column by column; then the factors.
    [...b.f.sources].reverse().forEach((s, i) => {
      const cx = x0 + (PAD + Math.floor(i / rows) + 0.5) * c;
      const cy = top + (PAD + (i % rows) + 0.5) * c;
      if (keptSet.has(s)) kept += circle(cx, cy, 0.4 * c);
      else rejected += circle(cx, cy, 0.32 * c);
    });
    for (let i = 0; i < b.f.factors; i++) {
      const cx = x0 + (PAD + b.sourceCols + 1 + Math.floor(i / rows) + 0.5) * c;
      const cy = top + (PAD + (i % rows) + 0.5) * c;
      factors += square(cx, cy, 0.3 * c);
    }
  }
  el('path', { d: factors, class: 'row-factor' }, svg);
  el('path', { d: kept, class: 'row-kept' }, svg);
  el('path', { d: rejected, class: 'row-rejected' }, svg);
  // The layer under each layer's frames.
  const labelY = top + boxH + 18;
  const byLayer = new Map();
  for (const b of boxes) byLayer.set(b.f.layer, [...(byLayer.get(b.f.layer) ?? []), b]);
  for (const [layer, list] of byLayer) {
    const a = left + list[0].x * c;
    const z = left + (list[list.length - 1].x + list[list.length - 1].w) * c;
    const t = el('text', { x: ((a + z) / 2).toFixed(1), y: labelY, 'text-anchor': 'middle', class: 'tick' }, svg);
    t.textContent = String(layer);
  }
  // The hover layer: a frame's counts.
  for (const b of boxes) {
    const x0 = left + b.x * c;
    const hit = el('rect', { x: (x0 - 1).toFixed(2), y: 0, width: (b.w * c + 2).toFixed(2), height: top + boxH + 4, fill: 'transparent', class: 'hit' }, svg);
    hit.addEventListener('pointerenter', () => {
      const rows = [`A frame of layer ${b.f.layer}`, ['sources', fmt(b.f.sources.length)]];
      if (data.decided) rows.push(['kept by greedy', fmt(b.f.kept.length)]);
      rows.push(['factors', fmt(b.f.factors)]);
      if (data.decided) rows.push(['freedoms', b.f.closed ? 'none: closed' : fmt(b.f.freedoms)]);
      tip.show(rows, x0 + (b.w * c) / 2, top + 20);
    });
    hit.addEventListener('pointerleave', () => tip.hide());
  }
}

/** The figure's facts once descending greedy has decided: the counts its caption says. */
export function rowFacts(frames) {
  const closed = frames.filter((f) => f.closed);
  return {
    frames: frames.length,
    closedLayers: [...new Set(closed.map((f) => f.layer))].sort((a, b) => a - b),
    open: frames.length - closed.length,
    freedoms: frames.reduce((a, f) => a + f.freedoms, 0),
  };
}

/** Mount the figure in host, which holds .row-box and .fig-caption.  Resolves with its facts once greedy has decided. */
export function mountFramesRow(host, { games, n = games.lastN }) {
  const box = host.querySelector('.row-box');
  const caption = host.querySelector('.fig-caption');
  const i = games.columns.n.indexOf(n);
  const recorded = games.columns.inTheBag[i];
  const candidates = games.columns.candidates[i];
  const worker = new Worker(new URL('./frames-row-worker.js', import.meta.url), { type: 'module' });
  worker.postMessage({ n });
  const data = { n, decided: false, frames: M.frames(n).map((f) => ({ layer: f.layer, sources: f.sources, factors: f.factors.length, kept: null })) };
  const tip = tooltip(box);
  let width = 0;
  const redraw = () => draw(box, tip, data, width);
  new ResizeObserver(() => {
    const w = Math.round(box.clientWidth);
    if (w > 0 && w !== width) {
      width = w;
      redraw();
    }
  }).observe(box);
  const numbers = data.frames.reduce((a, f) => a + f.sources.length + f.factors, 0);
  const describe = (rest) => `The ${data.frames.length} frames of game ${fmt(n)}, a mark for each of its ${fmt(numbers)} numbers: in each frame its sources at the left, gray where descending greedy keeps them and hollow where it does not, and its factors at the right. ${rest}`;
  caption.textContent = describe('Descending greedy is still working out which sources it keeps…');
  // The table, its last columns filled in once greedy has decided.
  const pendingRows = drawOrder(data.frames).map((f) => [String(f.layer), fmt(f.sources.length), '…', fmt(f.factors), '…']);
  tableView(host, 'The numbers behind the figure', ['layer', 'sources', 'kept', 'factors', 'freedoms'], pendingRows);
  return new Promise((resolve, reject) => {
    worker.onerror = (e) => {
      caption.textContent = `The frames of game ${fmt(n)}: descending greedy could not be run in this browser.`;
      reject(e);
    };
    worker.onmessage = ({ data: answer }) => {
      worker.terminate();
      if (answer.inTheBag !== recorded) {
        caption.textContent = `Game ${fmt(n)}: descending greedy does not give the recorded sum of U*.`;
        reject(new Error(`game ${n}: U* sums to ${answer.inTheBag}, and the results file records ${recorded}`));
        return;
      }
      data.frames = answer.frames;
      data.decided = true;
      if (width > 0) redraw();
      const facts = rowFacts(data.frames);
      caption.textContent = describe(`The frames of layers ${listWords(facts.closedLayers)} are closed. The other ${facts.open} have ${fmt(facts.freedoms)} freedoms in all, and ${fmt(candidates)} candidates.`);
      const cells = [...host.querySelectorAll('.chart-table tbody tr')];
      drawOrder(data.frames).forEach((f, j) => {
        const td = cells[j].children;
        td[2].textContent = fmt(f.kept.length);
        td[4].textContent = f.closed ? 'closed' : fmt(f.freedoms);
      });
      resolve(facts);
    };
  });
}
