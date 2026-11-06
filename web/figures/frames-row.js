// A row of a large game's frames, drawn small, to show the scale: every frame of the last game of the chain as a box,
// a mark for each of its numbers, the sources at its left (gray if descending greedy keeps them, hollow if not) and
// its factors at its right; a dashed box is a closed frame.  The frames stand in the wiki's order, the highest layer
// at the left and the frame in front at the right, and are named by their numbers (js/model.js, frameModel), frame 0
// being the one in front; a closed frame has no number and is named closed.  Frames with freedoms that share a number
// are one frame: once greedy has decided, they are drawn as a single box around all their numbers (collapseFrames).
//
// The frames come from js/model.js at once (frames), and what descending greedy keeps a moment later, from a worker
// (figures/frames-row-worker.js, frameModel), since descending greedy on a board of thousands takes seconds.  The
// figure is laid out from the frames alone, so it keeps its size when the worker's answer fills it in, and it checks
// the sum of U* against the recorded in_the_bag of data/games.json.  Every frame's box holds its numbers in columns of
// the same height, chosen so that the row fits the width it has as a band, not a tower.

import * as M from '../js/model.js';
import { el, fmt } from '../js/vocab.js';
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

// A frame's name: its number, or closed.  The tooltip's version says Frame h or A closed frame.
const nameOf = (u, title = false) => {
  if (u.closed) return title ? 'A closed frame' : 'closed';
  return title ? `Frame ${u.number}` : String(u.number);
};

const sum = (frames, count) => frames.reduce((a, f) => a + count(f), 0);
// The counts of a frame of the figure: the sums over the frames that make it up.
const countsOf = (frames) => ({
  sources: sum(frames, (f) => f.sources.length), kept: sum(frames, (f) => f.kept?.length ?? 0),
  factors: sum(frames, (f) => f.factors), freedoms: sum(frames, (f) => f.freedoms ?? 0),
});

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
  const svg = freshSVG(box, width, height, `The frames of game ${fmt(data.n)}, a mark for each number. The numbers are in the table below.`);
  svg.classList.add('frames-row-svg');
  // The frames as the reader names them: once greedy has decided, the frames of one number are one, a single box from
  // the first of them to the last; until then every frame stands alone.
  const boxOf = new Map(boxes.map((b) => [b.f, b]));
  const units = (data.decided ? M.collapseFrames(frames) : frames.map((f) => ({ closed: false, frames: [f] })))
    .map((u) => {
      const own = u.frames.map((f) => boxOf.get(f));
      const x0 = left + own[0].x * c;
      const x1 = left + (own[own.length - 1].x + own[own.length - 1].w) * c;
      return { ...u, x0, x1 };
    });
  for (const u of units) {
    el('rect', {
      x: u.x0.toFixed(2), y: top, width: (u.x1 - u.x0).toFixed(2), height: boxH.toFixed(2), rx: Math.min(6, 1.4 * c),
      class: u.closed ? 'row-frame is-closed' : 'row-frame',
    }, svg);
  }
  let kept = '';
  let rejected = '';
  let factors = '';
  for (const b of boxes) {
    const x0 = left + b.x * c;
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
  // The name under each frame: its number, or closed.  Until greedy has decided, no frame has one.  A name stands
  // where it fits: as wide as its box and half the space to each neighbor.  A number always shows, and a closed
  // frame, which is dashed, goes without a name that would run into its neighbors.
  const labelY = top + boxH + 18;
  const space = (u, i) => {
    const before = i > 0 ? u.x0 - units[i - 1].x1 : LAYER_GAP * c;
    const after = i < units.length - 1 ? units[i + 1].x0 - u.x1 : LAYER_GAP * c;
    return u.x1 - u.x0 + (before + after) / 2;
  };
  if (data.decided) {
    units.forEach((u, i) => {
      const x = ((u.x0 + u.x1) / 2).toFixed(1);
      const t = el('text', { x, y: labelY, 'text-anchor': 'middle', class: 'tick' }, svg);
      t.textContent = nameOf(u);
      if (u.closed && t.getBBox().width > space(u, i)) t.remove();
    });
  }
  // The hover layer: a frame's counts.
  for (const u of units) {
    const hit = el('rect', {
      x: (u.x0 - 1).toFixed(2), y: 0, width: (u.x1 - u.x0 + 2).toFixed(2), height: top + boxH + 4, fill: 'transparent',
      class: 'hit',
    }, svg);
    hit.addEventListener('pointerenter', () => {
      const counts = countsOf(u.frames);
      const rows = [data.decided ? nameOf(u, true) : 'A frame'];
      if (u.frames.length > 1) rows.push(['frames collapsed into it', fmt(u.frames.length)]);
      rows.push(['sources', fmt(counts.sources)]);
      if (data.decided) rows.push(['kept by greedy', fmt(counts.kept)]);
      rows.push(['factors', fmt(counts.factors)]);
      if (data.decided) rows.push(['freedoms', u.closed ? 'none: closed' : fmt(counts.freedoms)]);
      tip.show(rows, (u.x0 + u.x1) / 2, top + 20);
    });
    hit.addEventListener('pointerleave', () => tip.hide());
  }
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
  const describe = (rest) => `The frames of game ${fmt(n)}, a mark for each of its ${fmt(numbers)} numbers: in each frame its sources at the left, gray where descending greedy keeps them and hollow where it does not, and its factors at the right. ${rest}`;
  caption.textContent = describe('Descending greedy is still working out which sources it keeps…');
  // The table: a row for each frame, its last columns filled in once greedy has decided.
  const table = (rows) => {
    const old = host.querySelector('.chart-table');
    tableView(host, 'The numbers behind the figure', ['frame', 'sources', 'kept', 'factors', 'freedoms'], rows);
    if (old) {
      host.querySelector('.chart-table:last-of-type').open = old.open;
      old.remove();
    }
  };
  table(drawOrder(data.frames).map((f) => ['…', fmt(f.sources.length), '…', fmt(f.factors), '…']));
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
      const facts = M.frameFacts(data.frames);
      caption.textContent = describe(
        `Its ${facts.open} frames with freedoms collapse into frames 0 to ${facts.lastNumber}, and ${facts.closed} ` +
        `more are closed, dashed.  In all they have ${fmt(facts.freedoms)} freedoms and ${fmt(candidates)} candidates.`
      );
      table(M.collapseFrames(drawOrder(data.frames)).map((u) => {
        const counts = countsOf(u.frames);
        return [u.closed ? 'closed' : String(u.number), fmt(counts.sources), fmt(counts.kept), fmt(counts.factors),
          u.closed ? 'closed' : fmt(counts.freedoms)];
      }));
      resolve(facts);
    };
  });
}
