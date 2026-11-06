// The visual vocabulary of the site: one grammar, two renderings.  Every figure draws its numbers, lines and frames
// with the functions here, and the legend and the style page draw each mark with them too, so a reader learns the
// marks once.
//
// The grammar is the theory's (docs/theory/img/draw.py):
// * a circle is a move or a source, a square a factor; a promotion turns a square into a circle;
// * a gray fill is kept, a hollow dashed outline rejected, a double outline a candidate;
// * a thin line joins a number to one of its maximal factors, a thick line is a payment, and a dotted arrow runs from
//   a factor to its maximal factor in the frame in front;
// * a box is a frame, dashed when it is closed;
// * blue is the player's and orange the taxman's, and nothing else; what the prose names wears a halo of ink.
//
// The rule that joins the two renderings: depth means you can touch it.  A number the reader plays is a raised token
// (a coin for a circle, a tile for a square) standing on the page with an edge and a shadow, and one that cannot be
// taken lies flat.  A number in an explanatory figure is drawn flat, in ink.  Both renderings read the same look
// (NUMBER below), so the same look means the same thing in either.
//
// Colors are the custom properties of css/site.css, which gives the player's and the taxman's in three tones.  They
// are mixed in OKLab, so that a look passes smoothly from one value to another while a figure moves.  A color
// distinction is always doubled by shape, line style, position or a label.

export const NS = 'http://www.w3.org/2000/svg';

/** An SVG element with attributes, appended to parent when one is given. */
export function el(name, attrs = {}, parent = null) {
  const node = document.createElementNS(NS, name);
  for (const [key, value] of Object.entries(attrs)) node.setAttribute(key, value);
  if (parent) parent.appendChild(node);
  return node;
}

/** Sizes in the units of a figure: a circle's radius, a square's half side, the gap to a candidate's second outline,
 *  the gap from an outline to the end of a line, a move's tag, an arrowhead, and the edge of a raised token. */
export const SIZE = { circle: 15, square: 13, ring: 4.5, gap: 2, tag: 8.5, head: 8, depth: 3 };

/** The shape axis of a number: 0 a square, 1 a circle, halfway a plain number, one the story has not yet sorted into
 *  source or factor.  A promotion moves a number from 0 to 1. */
export const SHAPE = { square: 0, plain: 0.5, circle: 1 };

// ---------------------------------------------------------------------------------------------------------------
// Motion
// ---------------------------------------------------------------------------------------------------------------

const clamp01 = (t) => Math.min(1, Math.max(0, t));

/** The motion of the site.  Flat marks glide, in and out of their move; raised tokens are picked up, carried and set
 *  down with a small overshoot, as a thing with weight is.  The style page shows each.  Durations in milliseconds. */
export const MOTION = {
  step: 900, // one state of a stepped figure, played forward
  jump: (states) => Math.min(1100, 650 + 120 * states), // a jump over several states, all marks at once
  pause: 650, // the rest at each state while a figure plays through
  move: 620, // a raised token taken in the game, to its tray
  lift: 12, // the most a token is lifted while it travels, in figure units
  flipLift: 18, // the lift of a token turning over
  glide: (t) => (t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2), // cubic in and out
  settle: (t) => { // out, about 3 percent past the target and back
    const s = 1.05;
    const u = t - 1;
    return 1 + (s + 1) * u * u * u + s * u * u;
  },
  smooth: (t) => t * t * (3 - 2 * t),
  // The same two curves for CSS and the Web Animations API.
  glideCSS: 'cubic-bezier(0.65, 0, 0.35, 1)',
  settleCSS: 'cubic-bezier(0.3, 1.22, 0.55, 1)',
};

/** Whether the reader asked for less motion. */
export function reducedMotion() {
  return typeof matchMedia !== 'undefined' && matchMedia('(prefers-reduced-motion: reduce)').matches;
}

// ---------------------------------------------------------------------------------------------------------------
// Looks
// ---------------------------------------------------------------------------------------------------------------

/** The look of a number, the same in both renderings.  The axes run from 0 to 1 and blend from state to state:
 *    shape   0 a square (a factor), 1 a circle (a move or a source), halfway a plain number (not yet sorted)
 *    kept    the gray fill of a number kept, or played
 *    dashed  hollow, with a dashed outline: rejected, or not played
 *    ring    the double outline of a candidate
 *    player  the player's (blue); taxman, the taxman's (orange)
 *    halo    a halo of ink and a heavier weight: what the prose beside the figure names
 *    faded   out of focus, as the closed frames are once they drop out
 *    depth   raised only: 1 standing, a token that can be taken; 0 lying flat, one that cannot
 *    lift    raised only: how far the token is held above the page, in figure units */
export const NUMBER = {
  x: 0, y: 0, scale: 1, shape: SHAPE.circle, kept: 0, dashed: 0, ring: 0, player: 0, taxman: 0, halo: 0, faded: 0,
  opacity: 1, depth: 1, lift: 0,
};

/** Named looks, to combine: look('factor', 'candidate', { x: 40 }). */
export const LOOKS = {
  source: { shape: SHAPE.circle },
  move: { shape: SHAPE.circle },
  factor: { shape: SHAPE.square },
  plain: { shape: SHAPE.plain }, // a number the story has not yet sorted into source or factor
  kept: { kept: 1 },
  rejected: { dashed: 1 },
  candidate: { ring: 1 },
  player: { player: 1 },
  taxman: { taxman: 1 },
  named: { halo: 1 },
  faded: { faded: 1 },
  lying: { depth: 0, faded: 0.55 }, // raised only: a number that cannot be taken now
  hidden: { opacity: 0 },
};

/** A look made of named looks and plain values, in order: look('factor', 'kept', { x: 10, y: 20 }). */
export function look(...parts) {
  const out = {};
  for (const part of parts) {
    if (typeof part === 'string') {
      if (!LOOKS[part]) throw new Error(`no look named ${part}`);
      Object.assign(out, LOOKS[part]);
    } else Object.assign(out, part);
  }
  return out;
}

/** The look of a line: draw is the share of it drawn from its start; sag, how far a string hangs below the straight
 *  line (raised figures hang their strings); bend, from 0 to 1, how far it leaves its start and reaches its end level,
 *  curving between, so that a line between two columns of numbers never crosses a number of either column. */
export const LINE_LOOK = { opacity: 1, draw: 1, halo: 0, faded: 0, sag: 0, bend: 0 };

/** The kinds of line.  Each differs by weight, tone and dash, never by color alone.
 *    thin   a number and one of its maximal factors
 *    pay    a payment: the factor that pays for a move
 *    link   a factor and its maximal factor in the frame in front, a dotted arrow
 *    faint  a line out of focus
 *    rule   a rule across a figure, such as the line at n/2 */
export const LINE = {
  thin: { width: 1.3, tone: '--mid', raisedTone: '--string' },
  pay: { width: 3.6, tone: '--ink' },
  link: { width: 1.9, tone: '--mid', dash: '0.1 4.8', cap: 'round', head: true },
  faint: { width: 1.2, tone: '--faint' },
  rule: { width: 1.2, tone: '--mid', dash: '5 4' },
};

/** The look of a frame's box: dashed when the frame is closed. */
export const BOX = { x: 0, y: 0, w: 10, h: 10, dashed: 0, halo: 0, faded: 0, opacity: 1 };
/** The look of a tag, which numbers a move, and of a line of figure text. */
export const TAG = { x: 0, y: 0, halo: 0, opacity: 1 };
export const TEXT = { x: 0, y: 0, halo: 0, opacity: 1 };

// ---------------------------------------------------------------------------------------------------------------
// Color
// ---------------------------------------------------------------------------------------------------------------

const v = (name) => `var(${name})`;
// A color t of the way from color a (any CSS color) to the custom property b.
function mix(a, b, t) {
  if (t <= 0.001) return a;
  if (t >= 0.999) return v(b);
  return `color-mix(in oklab, ${a}, ${v(b)} ${(t * 100).toFixed(1)}%)`;
}
const round = (x, d = 2) => Math.round(x * 10 ** d) / 10 ** d;
const lerp = (a, b, t) => a + (b - a) * t;

// The colors of a number in each rendering, from its look.
function flatColors(p) {
  const fill = mix(mix(mix(v('--paper'), '--kept', p.kept * (1 - p.dashed)), '--player-tint', p.player), '--taxman-tint', p.taxman);
  const stroke = mix(mix(mix(v('--ink'), '--faint', p.faded), '--player', p.player), '--taxman', p.taxman);
  const label = mix(mix(mix(v('--ink'), '--ink-3', p.faded), '--player-ink', p.player), '--taxman-ink', p.taxman);
  return { fill, stroke, label };
}

function raisedColors(p) {
  const face = mix(mix(mix(v('--paper'), '--kept-face', p.kept * (1 - p.dashed)), '--player-face', p.player), '--taxman-face', p.taxman);
  const rim = mix(mix(mix(mix(v('--rim'), '--kept-rim', p.kept), '--faint', p.faded * (1 - p.kept)), '--player-rim', p.player), '--taxman-rim', p.taxman);
  const side = mix(mix(mix(v('--side'), '--kept-side', p.kept), '--player-side', p.player), '--taxman-side', p.taxman);
  const label = mix(mix(mix(v('--ink'), '--ink-3', p.faded), '--player-on', p.player), '--taxman-on', p.taxman);
  return { face, rim, side, label };
}

// The size of a number's label, by its count of digits, in figure units.
const labelSize = (text) => ({ 1: 14, 2: 14, 3: 11.5 }[String(text).length] ?? 9.5);

// ---------------------------------------------------------------------------------------------------------------
// Flat numbers
// ---------------------------------------------------------------------------------------------------------------

/** Half the size of a number's shape: its radius as a circle, half its side as a square. */
export function halfSize(p) {
  return lerp(SIZE.square, SIZE.circle, p.shape) * (p.scale ?? 1);
}

/** How far from a number's center a line toward it, along the unit vector (ux, uy), stops. */
export function reach(p, ux, uy) {
  const h = halfSize(p);
  const square = h / Math.max(Math.abs(ux), Math.abs(uy), 1e-6);
  return lerp(square, h, p.shape) + SIZE.ring * p.ring * (p.scale ?? 1) + SIZE.gap;
}

function placeRect(rect, x, y, h, rx) {
  rect.setAttribute('x', round(x - h));
  rect.setAttribute('y', round(y - h));
  rect.setAttribute('width', round(2 * h));
  rect.setAttribute('height', round(2 * h));
  rect.setAttribute('rx', round(rx));
}

/** A flat number: a shape with a halo, a second outline, a dashed twin of its outline, and its label. */
export function numberMark(parent, label) {
  const g = el('g', { class: 'v-num', 'data-n': label }, parent);
  const halo = el('rect', { class: 'v-halo', fill: 'none' }, g);
  const ring = el('rect', { class: 'v-ring', fill: 'none' }, g);
  const body = el('rect', { class: 'v-body' }, g);
  const dash = el('rect', { class: 'v-dash', fill: 'none', 'stroke-dasharray': '4 3' }, g);
  const text = el('text', { class: 'v-label', 'text-anchor': 'middle', dy: '0.35em' }, g);
  text.textContent = String(label);
  return { kind: 'number', rendering: 'flat', g, halo, ring, body, dash, text, size: labelSize(label) };
}

/** Draw a flat number with the look p (see NUMBER). */
export function setNumber(mark, p) {
  const s = p.scale ?? 1;
  const h = halfSize(p);
  const rx = lerp(2 * s, h, p.shape);
  placeRect(mark.body, p.x, p.y, h, rx);
  placeRect(mark.dash, p.x, p.y, h, rx);
  const r = SIZE.ring * s;
  placeRect(mark.ring, p.x, p.y, h + r, rx + r * p.shape);
  placeRect(mark.halo, p.x, p.y, h + 1.5 * s, rx + 1.5 * s * p.shape);
  const c = flatColors(p);
  const width = (1.6 + 0.6 * Math.max(p.player, p.taxman) + 1.1 * p.halo) * s;
  mark.body.style.fill = c.fill;
  mark.body.style.stroke = c.stroke;
  mark.body.style.strokeOpacity = 1 - p.dashed;
  mark.body.style.strokeWidth = width;
  mark.dash.style.stroke = c.stroke;
  mark.dash.style.strokeOpacity = p.dashed;
  mark.dash.style.strokeWidth = width;
  mark.ring.style.stroke = c.stroke;
  mark.ring.style.strokeOpacity = p.ring;
  mark.ring.style.strokeWidth = (1.3 + 0.5 * p.halo) * s;
  mark.halo.style.strokeOpacity = 0.16 * p.halo;
  mark.halo.style.strokeWidth = 8 * s;
  mark.text.setAttribute('x', round(p.x));
  mark.text.setAttribute('y', round(p.y));
  mark.text.style.fill = c.label;
  mark.text.style.fontSize = `${round(mark.size * s, 2)}px`;
  mark.text.style.fontWeight = Math.round(500 + 250 * Math.max(p.halo, 0.6 * p.player));
  mark.g.style.opacity = p.opacity;
  mark.g.style.display = p.opacity < 0.002 ? 'none' : '';
}

// ---------------------------------------------------------------------------------------------------------------
// Raised tokens
// ---------------------------------------------------------------------------------------------------------------

let defsInstalled = false;

/** Put the soft shadow of the raised tokens into the page, once; every inline SVG of the page can use it. */
export function installDefs() {
  if (defsInstalled || typeof document === 'undefined') return;
  defsInstalled = true;
  const svg = el('svg', { width: 0, height: 0, 'aria-hidden': 'true', focusable: 'false' });
  svg.style.position = 'absolute';
  svg.style.overflow = 'hidden';
  const defs = el('defs', {}, svg);
  const soft = el('filter', { id: 'tm-soft', x: '-50%', y: '-50%', width: '200%', height: '200%' }, defs);
  el('feGaussianBlur', { stdDeviation: 1.6 }, soft);
  document.body.prepend(svg);
}

/** The three faces of a raised token, by the shape axis: a tile below 1/4, plain from 1/4 to 3/4, a coin from 3/4.
 *  Each gives the half side of its square and the corner radius of its parts; the coin is a circle. */
const FACES = ['tile', 'plain', 'coin'];
const CORNER = {
  tile: { side: 4.5, halo: 6, inner: 2.5, ring: 7.5 },
  plain: { side: 8, halo: 9.5, inner: 5, ring: 11 },
};

/** The face a shape shows. */
export const faceOf = (shape) => (shape < 0.25 ? 'tile' : shape < 0.75 ? 'plain' : 'coin');

function shapeParts(group, face) {
  const r = SIZE.circle;
  const h = face === 'plain' ? lerp(SIZE.square, SIZE.circle, SHAPE.plain) : SIZE.square;
  const parts = {};
  if (face === 'coin') {
    parts.halo = el('circle', { r: r + 1.5, fill: 'none' }, group);
    parts.side = el('circle', { r }, group);
    parts.face = el('circle', { r, class: 'tk-face' }, group);
    parts.inner = el('circle', { r: r - 3.4, fill: 'none', 'stroke-width': 0.9 }, group);
    parts.ring = el('circle', { r: r + SIZE.ring, fill: 'none' }, group);
  } else {
    const rect = (s, rx) => ({ x: -s, y: -s, width: 2 * s, height: 2 * s, rx });
    const c = CORNER[face];
    parts.halo = el('rect', { ...rect(h + 1.5, c.halo), fill: 'none' }, group);
    parts.side = el('rect', rect(h, c.side), group);
    parts.face = el('rect', { ...rect(h, c.side), class: 'tk-face' }, group);
    parts.inner = el('rect', { ...rect(h - 3.2, c.inner), fill: 'none', 'stroke-width': 0.9 }, group);
    parts.ring = el('rect', { ...rect(h + SIZE.ring, c.ring), fill: 'none' }, group);
  }
  return parts;
}

/** A raised token for the number n, as an SVG group to place in a figure; setToken gives it its look. */
export function token(n) {
  installDefs();
  const root = el('g', { class: 'tk', 'data-n': n });
  // The shadow stays on the page while the token above it is lifted: by a move, a flip, or the prose naming it.
  const shadow = el('ellipse', { rx: SIZE.circle * 0.92, ry: SIZE.circle * 0.48, fill: '#000', filter: 'url(#tm-soft)' }, root);
  const hover = el('g', { class: 'tk-hover' }, root);
  const body = el('g', { class: 'tk-body' }, hover);
  const groups = {};
  const parts = {};
  for (const face of FACES) {
    groups[face] = el('g', {}, body);
    parts[face] = shapeParts(groups[face], face);
  }
  const label = el('text', { 'text-anchor': 'middle', dy: '0.35em', class: 'tk-num' }, body);
  label.textContent = String(n);
  label.style.fontSize = `${labelSize(n)}px`;
  root._tk = { n, shadow, hover, body, groups, parts, label, face: null };
  return root;
}

/** Draw a raised token with the look p (see NUMBER), and the two values a move adds on the way: hop, how far it is
 *  lifted, and flip, from 1 (face up) to 0 (edge on), where it turns over. */
export function setToken(node, p) {
  const t = node._tk;
  const s = p.scale ?? 1;
  const face = faceOf(p.shape);
  node.setAttribute('transform', `translate(${round(p.x)} ${round(p.y)}) scale(${round(s, 3)})`);
  node.style.opacity = round(p.opacity, 3);
  node.style.display = p.opacity < 0.002 ? 'none' : '';
  if (t.face !== face) {
    t.face = face;
    for (const f of FACES) t.groups[f].style.display = f === face ? '' : 'none';
  }
  const lift = (p.lift ?? 0) + (p.hop ?? 0);
  const flip = Math.max(0.04, p.flip ?? 1);
  t.body.setAttribute('transform', `translate(0 ${round(-lift)}) scale(${round(flip, 3)} 1)`);
  const depth = SIZE.depth * (p.depth ?? 1);
  const c = raisedColors(p);
  const parts = t.parts[face];
  // The halo of ink round the face, as a flat number wears it.
  parts.halo.style.stroke = v('--ink');
  parts.halo.style.strokeWidth = 8;
  parts.halo.style.strokeOpacity = round(0.16 * p.halo, 3);
  parts.side.setAttribute('transform', `translate(0 ${round(depth)})`);
  parts.side.style.fill = c.side;
  parts.side.style.opacity = round(Math.min(1, 0.25 + p.depth), 3);
  parts.face.style.fill = c.face;
  parts.face.style.stroke = c.rim;
  parts.face.style.strokeWidth = 1.4 + 0.6 * p.kept + 1.2 * p.halo;
  parts.face.setAttribute('stroke-dasharray', p.dashed > 0.5 ? '3.6 2.6' : 'none');
  parts.inner.style.stroke = c.rim;
  parts.inner.style.opacity = round(0.32 * (1 - p.dashed), 3);
  parts.ring.style.stroke = v('--ink');
  parts.ring.style.strokeWidth = 1.5;
  parts.ring.style.opacity = round(p.ring, 3);
  t.label.style.fill = c.label;
  t.label.style.fontWeight = Math.round(650 + 150 * p.halo);
  // The shadow is sharp under a token standing on the page, and grows and softens as the token is lifted.
  const up = Math.min(1, lift / 14);
  t.shadow.setAttribute('cy', round(depth + 3 + 6 * up));
  t.shadow.setAttribute('rx', round(SIZE.circle * 0.92 * flip * (1 + 0.15 * up)));
  t.shadow.style.opacity = round((0.12 - 0.05 * up) * (p.depth ?? 1) + 0.015, 3);
}

/** A token's mark object for the scene: the node, drawn by setToken. */
export function tokenMark(parent, label) {
  const node = token(label);
  parent.appendChild(node);
  return { kind: 'number', rendering: 'raised', g: node, node };
}

// ---------------------------------------------------------------------------------------------------------------
// Lines, boxes, tags, text
// ---------------------------------------------------------------------------------------------------------------

/** A line's mark in one of the LINE styles, with an arrowhead when the style has one.  A raised line hangs as a
 *  string, and its thin style is lighter, so the tokens stay the loud thing. */
export function lineMark(parent, style, rendering = 'flat') {
  const spec = LINE[style];
  if (!spec) throw new Error(`no line style ${style}`);
  const g = el('g', { class: `v-line v-line-${style}` }, parent);
  const path = el('path', { fill: 'none' }, g);
  if (spec.dash) path.setAttribute('stroke-dasharray', spec.dash);
  path.setAttribute('stroke-linecap', spec.cap || (rendering === 'raised' ? 'round' : 'butt'));
  const head = spec.head ? el('path', {}, g) : null;
  return { kind: 'line', style, spec, rendering, g, path, head };
}

// The part of the cubic curve [p0, p1, p2, p3] from its start to the share t of the way, as a cubic of its own.
function firstPart([p0, p1, p2, p3], t) {
  const at = (a, b) => [lerp(a[0], b[0], t), lerp(a[1], b[1], t)];
  const a = at(p0, p1);
  const b = at(p1, p2);
  const c = at(p2, p3);
  const ab = at(a, b);
  const bc = at(b, c);
  return [p0, a, ab, at(ab, bc)];
}

/** Draw a line from a to b, each a point { x, y } or a number's look, which the line stops short of. */
export function setLine(mark, a, b, p) {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const length = Math.hypot(dx, dy) || 1;
  const ux = dx / length;
  const uy = dy / length;
  const startGap = 'shape' in a ? reach(a, ux, uy) : 0;
  let endGap = 'shape' in b ? reach(b, -ux, -uy) : 0;
  if (mark.head) endGap += SIZE.head;
  const full = Math.max(0, length - startGap - endGap);
  const x1 = a.x + ux * startGap;
  const y1 = a.y + uy * startGap;
  let x2 = x1 + ux * full * p.draw;
  let y2 = y1 + uy * full * p.draw;
  let hx = ux; // the direction of the line where it ends, for the arrowhead
  let hy = uy;
  const tone = mark.rendering === 'raised' && mark.spec.raisedTone ? mark.spec.raisedTone : mark.spec.tone;
  const color = mix(mix(v(tone), '--faint', p.faded), '--ink', 0.75 * p.halo);
  const bend = p.bend ?? 0;
  if (bend > 0.001) {
    // A bent line runs from the side of its start that faces its end to the side of its end that faces its start,
    // leaving and arriving level, as a cubic whose handles are level; drawn in part, it is the cubic's first part.
    const along = Math.sign(dx) || 1;
    const sideGap = 'shape' in a ? reach(a, along, 0) : 0;
    const endSide = ('shape' in b ? reach(b, -along, 0) : 0) + (mark.head ? SIZE.head : 0);
    const p0 = [lerp(x1, a.x + along * sideGap, bend), lerp(y1, a.y, bend)];
    const p3 = [lerp(x1 + ux * full, b.x - along * endSide, bend), lerp(y1 + uy * full, b.y, bend)];
    const handle = 0.5 * bend * (p3[0] - p0[0]);
    let curve = [p0, [p0[0] + handle, p0[1]], [p3[0] - handle, p3[1]], p3];
    if (p.draw < 0.999) curve = firstPart(curve, p.draw);
    const [q0, q1, q2, q3] = curve;
    const xy = ([x, y]) => `${round(x)} ${round(y)}`;
    mark.path.setAttribute('d', `M${xy(q0)}C${xy(q1)} ${xy(q2)} ${xy(q3)}`);
    [x2, y2] = q3;
    const tx = q3[0] - q2[0];
    const ty = q3[1] - q2[1];
    const t = Math.hypot(tx, ty);
    if (t > 1e-6) [hx, hy] = [tx / t, ty / t];
  } else {
    // A string sags below the straight line, the way a string hangs; a straight line has no sag.
    const sag = (p.sag ?? 0) * Math.min(1, full / 90);
    const side = ux >= 0 ? 1 : -1;
    const cx = (x1 + x2) / 2 - uy * sag * side;
    const cy = (y1 + y2) / 2 + ux * sag * side;
    mark.path.setAttribute('d', sag > 0.05
      ? `M${round(x1)} ${round(y1)}Q${round(cx)} ${round(cy)} ${round(x2)} ${round(y2)}`
      : `M${round(x1)} ${round(y1)}L${round(x2)} ${round(y2)}`);
  }
  mark.path.style.stroke = color;
  mark.path.style.strokeWidth = mark.spec.width + (mark.style === 'pay' ? 0 : 0.9 * p.halo);
  if (mark.head) {
    const s = SIZE.head;
    const tipX = x2 + hx * s;
    const tipY = y2 + hy * s;
    const px = -hy * s * 0.45;
    const py = hx * s * 0.45;
    mark.head.setAttribute('d', `M${round(tipX)},${round(tipY)} L${round(x2 + px)},${round(y2 + py)} L${round(x2 - px)},${round(y2 - py)} Z`);
    mark.head.style.fill = color;
    mark.head.style.opacity = p.draw > 0.98 ? 1 : 0;
  }
  const visible = p.opacity * (full > 0.5 ? 1 : 0);
  mark.g.style.opacity = visible;
  mark.g.style.display = visible < 0.002 || p.draw < 0.002 ? 'none' : '';
}

/** A frame's box: a solid outline and a dashed twin, crossfaded as the frame opens or closes.  Raised, it is a tray. */
export function boxMark(parent, rendering = 'flat') {
  const g = el('g', { class: `v-box v-box-${rendering}` }, parent);
  const solid = el('rect', { rx: 11 }, g);
  const dash = el('rect', { fill: 'none', rx: 11, 'stroke-dasharray': '7 5' }, g);
  return { kind: 'box', rendering, g, solid, dash };
}

export function setBox(mark, p) {
  const raised = mark.rendering === 'raised';
  const tone = mix(mix(v(raised ? '--tray-edge' : '--ink'), '--faint', p.faded), '--ink', p.halo);
  for (const rect of [mark.solid, mark.dash]) {
    rect.setAttribute('x', round(p.x));
    rect.setAttribute('y', round(p.y));
    rect.setAttribute('width', round(Math.max(0, p.w)));
    rect.setAttribute('height', round(Math.max(0, p.h)));
    rect.style.stroke = tone;
    rect.style.strokeWidth = 1.4 + 1.0 * p.halo;
  }
  mark.solid.style.fill = raised ? v('--tray') : 'none';
  mark.solid.style.fillOpacity = raised ? 1 - 0.5 * p.dashed : 0;
  mark.solid.style.strokeOpacity = 1 - p.dashed;
  mark.dash.style.strokeOpacity = p.dashed;
  mark.g.style.opacity = p.opacity;
  mark.g.style.display = p.opacity < 0.002 ? 'none' : '';
}

/** A tag that numbers a move: a small circle with the move's place in the game. */
export function tagMark(parent, label) {
  const g = el('g', { class: 'v-tag' }, parent);
  const circle = el('circle', { r: SIZE.tag }, g);
  const text = el('text', { 'text-anchor': 'middle', dy: '0.35em' }, g);
  text.textContent = String(label);
  return { kind: 'tag', g, circle, text };
}

export function setTag(mark, p) {
  mark.circle.setAttribute('cx', round(p.x));
  mark.circle.setAttribute('cy', round(p.y));
  mark.circle.style.strokeWidth = 1.3 + 0.7 * p.halo;
  mark.text.setAttribute('x', round(p.x));
  mark.text.setAttribute('y', round(p.y));
  mark.g.style.opacity = p.opacity;
  mark.g.style.display = p.opacity < 0.002 ? 'none' : '';
}

/** A line of figure text.  kind is 'label' (a note), 'name' (the name of a frame or a layer) or 'score'. */
export function textMark(parent, kind = 'label', anchor = 'start') {
  const text = el('text', { class: `v-text v-text-${kind}`, 'text-anchor': anchor, dy: '0.35em' }, parent);
  return { kind: 'text', text };
}

export function setText(mark, p, content) {
  if (content !== undefined && mark.text.textContent !== content) mark.text.textContent = content;
  mark.text.setAttribute('x', round(p.x));
  mark.text.setAttribute('y', round(p.y));
  mark.text.style.fontWeight = Math.round(500 + 250 * p.halo);
  mark.text.style.fill = mix(v('--ink-2'), '--ink', p.halo);
  mark.text.style.opacity = p.opacity;
  mark.text.style.display = p.opacity < 0.002 ? 'none' : '';
}

// ---------------------------------------------------------------------------------------------------------------
// Marks on their own, for HTML: the contents, the game's pieces, the legend
// ---------------------------------------------------------------------------------------------------------------

/** A number as an inline SVG of its own.  rendering is 'raised' or 'flat'; lookParts are passed to look(); size is
 *  its width in CSS pixels; pad leaves room for a second outline or a lift. */
export function numberSVG(n, { rendering = 'raised', looks = [], size = 40, pad = 6, label = null } = {}) {
  installDefs();
  const half = SIZE.circle + pad;
  const extra = rendering === 'raised' ? SIZE.depth + 2 : 0;
  const svg = el('svg', {
    viewBox: `${-half} ${-half} ${2 * half} ${2 * half + extra}`, width: size,
    height: round((size * (2 * half + extra)) / (2 * half), 1), 'aria-hidden': 'true', focusable: 'false',
    class: `mark-svg mark-${rendering}`,
  });
  const p = { ...NUMBER, ...look(...looks) };
  if (rendering === 'raised') {
    const node = token(label ?? n);
    svg.appendChild(node);
    setToken(node, p);
    svg._token = node;
  } else {
    const mark = numberMark(svg, label ?? n);
    setNumber(mark, p);
    svg._mark = mark;
  }
  svg._look = p;
  return svg;
}

/** Redraw a mark made by numberSVG with a new look. */
export function restyle(svg, ...looks) {
  const p = { ...NUMBER, ...look(...looks) };
  if (svg._token) setToken(svg._token, p);
  else setNumber(svg._mark, p);
  svg._look = p;
}

// ---------------------------------------------------------------------------------------------------------------
// Standing: where a statement stands
// ---------------------------------------------------------------------------------------------------------------

/** The standings a statement can have.  The badge's outline and fill tell them apart: filled for proved, gray under a
 *  solid outline for proved by computation, a double outline for proved given U, gray stripes for open but proved
 *  in every game up to some N, dotted for measured, dashed for
 *  open. */
export const STANDING = {
  proved: 'proved',
  computed: 'proved by computation',
  'given-u': 'proved given U',
  certified: 'open, proved to game N',
  measured: 'measured',
  open: 'open',
};

/** The badge of a standing, as HTML; with an href it is the link to the proof. */
export function standingBadge(kind, href) {
  if (!STANDING[kind]) throw new Error(`no standing ${kind}`);
  if (href) return `<a class="standing is-${kind}" href="${href}">${STANDING[kind]}<span class="sr-only">, ${kind === 'measured' || kind === 'open' ? 'evidence' : 'proof'} on GitHub</span></a>`;
  return `<span class="standing is-${kind}">${STANDING[kind]}</span>`;
}

// ---------------------------------------------------------------------------------------------------------------
// Words
// ---------------------------------------------------------------------------------------------------------------

/** A number written for reading: 218226 as 218,226. */
export function fmt(n) {
  return Number(n).toLocaleString('en-US');
}

/** A time in seconds, to two significant figures in the unit that suits: 0.0011 s as 1.1 ms, 16,094 s as 4.5 h, and
 *  from two days on in days. */
export function duration(s) {
  if (s == null) return '—';
  const two = (x) => fmt(Number(x.toPrecision(2)));
  if (s < 1) return `${two(s * 1000)} ms`;
  if (s < 60) return `${two(s)} s`;
  if (s < 3600) return `${two(s / 60)} min`;
  if (s < 2 * 86400) return `${two(s / 3600)} h`;
  return `${two(s / 86400)} days`;
}

/** A list in words: [11, 13, 17] as "11, 13 and 17". */
export function listWords(items) {
  const xs = items.map(String);
  if (xs.length <= 1) return xs.join('');
  return `${xs.slice(0, -1).join(', ')} and ${xs[xs.length - 1]}`;
}

export { clamp01, lerp, round };
