// The drawing of an odd core, shared by chapter 6's figures of game 45 (figures/core-45.js) and game 78
// (figures/exact-78.js): the numbers of the core on a ring of the factors they list, each number between the factors
// it can pay with and the ones that list a third joined across the ring to it, or in layers (the numbers at the left,
// the factors at the right), with the "Show: as a ring / in layers" switch of figures/layer-view.js.  The numbers are
// gray circles, in the set being tested, a promotion of the set wears a double outline, and the factors are squares.
//
// The caller gives the facts (the core's numbers with the factors each lists) and the order of the ring, written by
// hand, or the places of the ring view where another figure has already drawn the core in its own shape; this module
// writes the caption of each view, which says only what is particular to the core, since the notation page explains
// the marks.  Each number of a core lists two or three factors, which is checked.  The ring is checked against the
// facts: every entry is in the core, each number stands between two factors it lists, and the numbers are all one
// layer, with the factors in the layer below.

import * as M from '../js/model.js';
import { SIZE, listWords } from '../js/vocab.js';
import { scene } from '../js/scene.js';
import { holdHeight } from './hold-height.js';
import { CLEAR, layerColumns, mountViewSwitch, viewFromUrl } from './layer-view.js';

// The layers: the pitch between the numbers at the left.  It keeps a promotion's double outline CLEAR units clear of
// the circle beside it at the marks' full size, the size they have on the ring; the extra SIZE.ring is what the shared
// clearance does not cover.
const PITCH = 2 * SIZE.circle + SIZE.ring + CLEAR;

/** Mount a core of game n in host, which holds an <svg> and a .fig-caption.  ring lists each factor, then the number
 *  between it and the next, round the ring; lists is a Map from each number of the core to the factors it lists;
 *  promotions are the numbers that wear a double outline; size is { width, height, cx, cy, a, b }, the box of the ring
 *  view and the center and radii of its ellipse, or, with at, only { width, height }; at, if given, holds the place
 *  [x, y] of every number and factor in the ring view, in place of the ellipse; viewParam is the URL parameter that names the view.  Returns
 *  { s, k, sources, factors }. */
export function mountOddCore(host, { n, ring, lists, promotions = [], viewParam, size, at, label }) {
  const factors = [...new Set([...lists.values()].flat())].sort((a, b) => a - b);
  const onRing = new Set(ring);
  if (ring.length !== lists.size + factors.length || ![...lists.keys(), ...factors].every((x) => onRing.has(x))) {
    throw new Error(`game ${n}: the ring is not the core`);
  }
  ring.forEach((x, i) => {
    const sides = [ring[(i + ring.length - 1) % ring.length], ring[(i + 1) % ring.length]];
    if (lists.has(x) && !sides.every((g) => lists.get(x).includes(g))) {
      throw new Error(`game ${n}: ${x} is not between its factors`);
    }
  });
  const sources = ring.filter((x) => lists.has(x));
  const k = M.omega(sources[0]);
  if (!sources.every((x) => M.omega(x) === k) || !factors.every((x) => M.omega(x) === k - 1)) {
    throw new Error(`game ${n}: the core is not in one layer, with its factors in the layer below`);
  }
  const layers = layerColumns({
    // The layers put the largest number at the top, as the site's other layer views do.
    sources: [...sources].sort((x, y) => y - x), factors, factorsOf: (x) => lists.get(x), width: size.width, pitch: PITCH,
    maxScale: 1,
  });
  const boxes = {
    graph: { width: size.width, height: size.height },
    layers: { width: size.width, height: layers.height },
  };
  const view = viewFromUrl(viewParam);
  host.style.setProperty('--w', size.width); // every odd core at one scale (css/bound.css)
  const s = scene(host.querySelector('svg'), { ...boxes[view], className: 'is-dense odd-core', label });
  const ringLook = {};
  ring.forEach((x, i) => {
    const angle = -Math.PI / 2 + (2 * Math.PI * i) / ring.length;
    s.number(x, x);
    ringLook[x] = at
      ? { x: at[x][0], y: at[x][1] }
      : { x: size.cx + size.a * Math.cos(angle), y: size.cy + size.b * Math.sin(angle) };
  });
  const marks = {};
  for (const x of ring) {
    const inCore = lists.has(x) ? 1 : 0;
    marks[x] = { shape: inCore, kept: inCore, ring: promotions.includes(x) ? 1 : 0 };
  }
  const lines = {};
  for (const [x, list] of lists) {
    for (const g of list) {
      s.line(`${x}-${g}`, x, g, 'thin');
      lines[`${x}-${g}`] = {};
    }
  }
  const look = (places) => {
    const st = { ...lines };
    for (const x of ring) st[x] = { ...marks[x], ...places[x] };
    return st;
  };
  const views = { graph: [look(ringLook)], layers: [look(layers.places)] };
  s.states(views[view]).show(0);

  if (![...lists.values()].every((list) => list.length === 2 || list.length === 3)) {
    throw new Error(`game ${n}: a number of the core lists fewer than two factors or more than three`);
  }
  const threes = sources.filter((x) => lists.get(x).length === 3).sort((a, b) => a - b);
  const text = {
    graph: `The odd core of game ${n}. Each number sits between two factors it can pay with, and `
      + `${listWords(threes)} ${threes.length === 1 ? 'reaches' : 'reach'} across the ring to a third.`,
    layers: `The odd core of game ${n}, in its layers: the numbers of layer ${k} at the left, the factors they can `
      + `pay with, of layer ${k - 1}, at the right.`,
  };
  const caption = host.querySelector('.fig-caption');
  holdHeight(caption, Object.values(text));
  caption.textContent = text[view];
  mountViewSwitch(host, s, {
    view, lists: views, boxes, param: viewParam, names: { graph: 'as a ring', layers: 'in layers' },
    onView: (v) => { caption.textContent = text[v]; },
  });
  return { s, k, sources, factors };
}
