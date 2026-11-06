// The two views of a figure whose numbers can be drawn as a graph or in their layers: the "Show: as a graph / in
// layers" switch, and the layout of the layers, shared by the figures of game 100's layer 2 and game 2,873's six
// sources.
//
// The layers are columns, as chapter 2 draws them: the sources at the left, in the order the figure gives, their
// maximal factors at the right, in the order of the mean place of their sources so that few lines cross, and a line
// runs rightward from each source to each of its factors.  The columns are as long as the sources need, so the layers
// view can be taller than the graph; switching glides every mark to its place and the box grows or shrinks with it.

import { SIZE, reducedMotion } from '../js/vocab.js';

const VIEWS = {
  graph: 'as a graph',
  layers: 'in layers',
};

/** The view a URL parameter names: 'layers', or else the graph. */
export function viewFromUrl(param) {
  return new URLSearchParams(location.search).get(param) === 'layers' ? 'layers' : 'graph';
}

/** The places of the layers in a box width wide: the sources in the left column, in the order given, and the factors
 *  in the right column, each column spread over the same length, with margin before the first number and after the
 *  last, pitch between the sources, and footer below for anything else.  The marks are as large as the pitch lets
 *  them be, up to maxScale.  factorsOf(s) lists the factors of source s.  Returns { places, height }. */
export function layerColumns({ sources, factors, factorsOf, width, pitch, margin = 24, footer = 0, maxScale = 1 }) {
  const length = (sources.length - 1) * pitch;
  const scale = Math.min(maxScale, (0.95 * pitch) / (2 * SIZE.circle));
  const meanPlace = (x) => {
    const mine = sources.map((s, i) => (factorsOf(s).includes(x) ? i : null)).filter((i) => i !== null);
    return mine.reduce((a, b) => a + b, 0) / mine.length;
  };
  const right = [...factors].sort((a, b) => meanPlace(a) - meanPlace(b) || b - a);
  const places = {};
  const column = (list, x) => list.forEach((id, i) => {
    places[id] = { x, y: margin + (list.length === 1 ? length / 2 : (i * length) / (list.length - 1)), scale };
  });
  column(sources, width / 2 - 0.3 * width);
  column(right, width / 2 + 0.3 * width);
  return { places, height: 2 * margin + length + footer };
}

/** The switch between the views: a button for each, the one shown pressed. */
function viewSwitch(onPick) {
  const box = document.createElement('div');
  box.className = 'fig-view';
  box.setAttribute('role', 'group');
  box.setAttribute('aria-label', 'View of the figure');
  box.innerHTML = `<span class="fig-view-word">Show</span>${Object.entries(VIEWS)
    .map(([view, name]) => `<button type="button" class="btn" data-view="${view}">${name}</button>`).join('')}`;
  const buttons = [...box.querySelectorAll('button')];
  for (const b of buttons) b.addEventListener('click', () => onPick(b.dataset.view));
  const show = (view) => buttons.forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.view === view)));
  return { box, show };
}

// Glide the svg's box to height h over ms milliseconds, from wherever it is.
const glides = new WeakMap();
function growTo(svg, width, h, ms) {
  cancelAnimationFrame(glides.get(svg));
  const from = svg.viewBox.baseVal.height;
  const start = performance.now();
  const frame = (now) => {
    const t = ms > 0 ? Math.min(1, (now - start) / ms) : 1;
    svg.setAttribute('viewBox', `0 0 ${width} ${from + (h - from) * t}`);
    if (t < 1) glides.set(svg, requestAnimationFrame(frame));
  };
  frame(start);
}

/** Put the switch above the scene s in host and wire it.  lists[view] is the scene's states in each view, boxes[view]
 *  its { width, height }; param is the URL parameter that opens the figure in layers; onView(view), if given, is
 *  called when the view changes.  The scene's box is the initial view's. */
export function mountViewSwitch(host, s, { view, lists, boxes, param, onView }) {
  const switcher = viewSwitch((to) => {
    if (to === view) return;
    s.stop();
    const k = Math.round(s.pos);
    view = to;
    s.states(lists[view]);
    const ms = reducedMotion() ? 0 : 800;
    s.morph(Object.fromEntries(s.looksOf(k)), { duration: ms });
    growTo(s.svg, boxes[view].width, boxes[view].height, ms);
    onView?.(view);
    switcher.show(view);
    const url = new URL(location.href);
    if (view === 'layers') url.searchParams.set(param, view);
    else url.searchParams.delete(param);
    history.replaceState(null, '', url);
  });
  switcher.show(view);
  host.insertBefore(switcher.box, s.svg);
}
