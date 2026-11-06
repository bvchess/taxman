// The scene helper: one figure, one set of marks, many hand-written states.
//
// A figure registers its marks once (numbers, lines, frames, tags, text), each with an id, and then hands the scene a
// list of states written by hand: each maps a mark's id to its look there (the looks of vocab.js).  A mark keeps its
// identity from state to state, so the reader follows the same 21 through every state.  A mark a state leaves out is
// hidden there, in the place of the nearest state that shows it, so it fades in and out where it stands.
//
// The scene draws any point between two states (2.4 is four tenths of the way from the third state to the fourth),
// so a slider can drag it, and it moves by itself from wherever it is to a state (go), or, in a figure the reader
// plays, to looks worked out from the move (morph).  Flat marks glide; raised tokens are lifted while they travel,
// turn over when their shape changes, and settle with a little overshoot (vocab.js, MOTION).  With
// prefers-reduced-motion the scene jumps.
//
// Four optional keys of a look change how a mark moves into it from the state before:
//   at:   [a, b]  the mark moves only between a and b of the step, so that marks can move one after another;
//   via:  {...}   its look halfway, for a mark that does something on the way, such as a brief halo;
//   keys: [[u, {...}], ...]  looks at points u of the step, each over the state's own look;
//   through: { x, y }  a place the mark passes through on its way, without stopping: it travels along the one smooth
//         curve (a quadratic Bezier) from its place to the look's, bent so that it crosses through at the curve's
//         middle, with the motion's own easing on the curve, so that there is one lift and one landing.  Unlike via,
//         it is a place, not a look, and the mark's other looks move as they do without it.  It works in morph and in
//         a state's step that has no via or keys (a step cut into parts has no one curve); there, a state's through
//         is ignored.
// A state may also carry _duration, the milliseconds of the step into it.

import * as v from './vocab.js';

const { MOTION, clamp01 } = v;
const SPECIAL = new Set(['at', 'via', 'keys', 'through']);

// Numbers glide from a to b; anything else switches halfway.
function lerpLook(a, b, t) {
  const out = {};
  for (const key of new Set([...Object.keys(a), ...Object.keys(b)])) {
    const x = a[key];
    const y = b[key];
    if (x === undefined) out[key] = y;
    else if (y === undefined) out[key] = x;
    else out[key] = typeof x === 'number' && typeof y === 'number' ? x + (y - x) * t : t < 0.5 ? x : y;
  }
  return out;
}

// The place at s of the way from a to b along the quadratic Bezier that crosses through at s = 0.5: its control point
// is 2 * through - (a + b) / 2.  Beyond 1 (a settling overshoot) the curve goes on, so the landing is the same.
function bend(a, b, s, through) {
  const at = (k) => {
    const c = 2 * through[k] - (a[k] + b[k]) / 2;
    return (1 - s) * (1 - s) * a[k] + 2 * s * (1 - s) * c + s * s * b[k];
  };
  return { x: at('x'), y: at('y') };
}

const blendFlat = (a, b, t, through) => {
  const s = MOTION.glide(t);
  return { ...lerpLook(a, b, s), ...(through && bend(a, b, s, through)) };
};

// A raised token is picked up, carried and set down: its place settles with an overshoot, it is lifted on the way,
// and a token whose shape changes turns over, edge on at the halfway point, where it changes face.
function blendRaised(a, b, t, through) {
  const s = MOTION.settle(t);
  const place = {
    ...lerpLook({ x: a.x, y: a.y, scale: a.scale }, { x: b.x, y: b.y, scale: b.scale }, s),
    ...(through && bend(a, b, s, through)),
  };
  const out = { ...lerpLook(a, b, MOTION.smooth(t)), ...place };
  const travel = Math.hypot(b.x - a.x, b.y - a.y);
  let hop = (a.hop ?? 0) * (1 - MOTION.smooth(t)) + Math.min(MOTION.lift, travel / 18) * Math.sin(Math.PI * t);
  out.flip = 1;
  if (v.faceOf(a.shape) !== v.faceOf(b.shape)) {
    out.flip = Math.abs(Math.cos(Math.PI * t));
    out.shape = t < 0.5 ? a.shape : b.shape; // the face changes at the halfway point, edge on
    hop += MOTION.flipLift * Math.sin(Math.PI * t);
  }
  out.hop = hop;
  return out;
}

function strip(spec) {
  const out = {};
  for (const [k, x] of Object.entries(spec)) if (!SPECIAL.has(k)) out[k] = x;
  return out;
}

/** A new scene.  target is an <svg> to draw in, or an element to put a new one in.  rendering is the scene's default,
 *  'flat' for an explanatory figure and 'raised' for one the reader plays; a mark can say otherwise.  label is the
 *  figure's text description. */
export function scene(target, { width, height, label = '', className = '', rendering = 'flat' }) {
  let svg = target;
  if (!(target instanceof SVGSVGElement)) {
    svg = v.el('svg', {});
    target.append(svg);
  }
  svg.setAttribute('viewBox', `0 0 ${width} ${height}`);
  svg.classList.add('v-figure', `is-${rendering}`);
  if (className) for (const c of className.split(' ')) svg.classList.add(c);
  if (!svg.hasAttribute('role')) svg.setAttribute('role', 'img');
  if (label) svg.setAttribute('aria-label', label);
  if (rendering === 'raised') v.installDefs();
  const layer = {};
  for (const name of ['box', 'line', 'number', 'tag', 'text', 'front']) layer[name] = v.el('g', { class: `v-layer-${name}` }, svg);

  const marks = new Map(); // id -> { kind, mark, defaults, rendering, from, to, content, apply }
  let states = [];
  let looks = []; // looks[s].get(id): { look, path, at }
  let durations = [];
  let current = null; // the looks last drawn
  let anim = null;
  let settleAnim = null;
  const listeners = [];

  function add(id, kind, mark, extra = {}) {
    if (marks.has(id)) throw new Error(`the scene already has a mark ${id}`);
    const defaults = extra.defaults ?? {
      number: v.NUMBER, line: v.LINE_LOOK, box: v.BOX, tag: v.TAG, text: v.TEXT,
    }[kind];
    marks.set(id, { kind, mark, defaults, rendering: mark.rendering ?? rendering, ...extra });
    return api;
  }

  const blendOf = (id) => {
    const m = marks.get(id);
    return m.kind === 'number' && m.rendering === 'raised' ? blendRaised : blendFlat;
  };

  // Every mark's look in every state: its defaults under what the state gives, and, where the state leaves the mark
  // out, the look of the nearest state that shows it, at opacity 0.
  function resolve() {
    looks = states.map(() => new Map());
    durations = states.map((s) => s._duration ?? MOTION.step);
    for (const [id, m] of marks) {
      const given = states.map((state) => state[id]);
      states.forEach((state, s) => {
        let spec = given[s];
        let hidden = false;
        if (!spec) {
          hidden = true;
          for (let d = 1; d < states.length && !spec; d++) spec = given[s - d] || given[s + d];
          spec = spec || {};
        }
        const look = { ...m.defaults, ...strip(spec) };
        if (hidden) look.opacity = 0;
        const entry = { look, at: hidden ? null : spec.at, through: hidden || spec.via || spec.keys ? null : spec.through };
        if (s > 0) entry.path = path(looks[s - 1].get(id).look, look, hidden ? {} : spec);
        looks[s].set(id, entry);
      });
    }
  }

  // The looks a mark passes through on its way from look a to look b, each at its point of the step.
  function path(a, b, spec) {
    const points = [[0, a]];
    if (spec.via) points.push([0.5, { ...lerpLook(a, b, 0.5), ...spec.via }]);
    for (const [u, part] of spec.keys || []) points.push([u, { ...b, ...part }]);
    points.push([1, b]);
    return points.sort((p, q) => p[0] - q[0]);
  }

  // The looks of every mark at position pos, along the staged paths of the step it lies in.
  function lookAt(pos) {
    const last = states.length - 1;
    const p = Math.max(0, Math.min(last, pos));
    const k = Math.min(Math.floor(p), Math.max(0, last - 1));
    const t = last === 0 ? 0 : p - k;
    const now = new Map();
    for (const [id] of marks) {
      const a = looks[k].get(id);
      const b = looks[Math.min(k + 1, last)].get(id);
      if (last === 0 || !b.path || t === 0) {
        now.set(id, t === 1 ? b.look : a.look);
        continue;
      }
      let local = t;
      if (b.at) local = clamp01((t - b.at[0]) / Math.max(1e-6, b.at[1] - b.at[0]));
      const points = b.path;
      let i = 0;
      while (i < points.length - 2 && local > points[i + 1][0]) i++;
      const [u0, from] = points[i];
      const [u1, to] = points[i + 1];
      now.set(id, blendOf(id)(from, to, clamp01((local - u0) / Math.max(1e-6, u1 - u0)), b.through));
    }
    return now;
  }

  // The looks between two sets of looks, all marks at once or each in its own window (at), a mark with a place in
  // throughs passing through it.
  function between(from, to, t, windows = {}, throughs = {}) {
    const now = new Map();
    for (const [id] of marks) {
      const a = from.get(id);
      const b = to.get(id);
      const w = windows[id];
      const local = w ? clamp01((t - w[0]) / Math.max(1e-6, w[1] - w[0])) : t;
      now.set(id, blendOf(id)(a, b, local, throughs[id]));
    }
    return now;
  }

  function draw(now, pos) {
    current = now;
    const endOf = (end) => (typeof end === 'object' ? end : now.get(end));
    for (const [id, m] of marks) {
      const look = now.get(id);
      if (m.kind === 'number') {
        if (m.rendering === 'raised') v.setToken(m.mark.node, look);
        else v.setNumber(m.mark, look);
      } else if (m.kind === 'line') v.setLine(m.mark, endOf(m.from), endOf(m.to), look);
      else if (m.kind === 'box') v.setBox(m.mark, look);
      else if (m.kind === 'tag') v.setTag(m.mark, look);
      else if (m.kind === 'text') v.setText(m.mark, look, typeof m.content === 'function' ? m.content(pos, look) : m.content);
      else if (m.kind === 'custom') m.apply(m.mark, look, pos);
    }
  }

  function emit(pos, settled) {
    for (const fn of listeners) fn(pos, settled);
  }

  function stop() {
    if (anim) cancelAnimationFrame(anim);
    anim = null;
    if (settleAnim) {
      const done = settleAnim;
      settleAnim = null;
      done();
    }
  }

  // Run frame(t) for t from 0 to 1 over ms milliseconds.
  function run(ms, frame) {
    stop();
    return new Promise((resolve) => {
      settleAnim = resolve;
      const start = performance.now();
      const step = (now) => {
        const t = clamp01((now - start) / ms);
        frame(t);
        if (t < 1) anim = requestAnimationFrame(step);
        else {
          anim = null;
          settleAnim = null;
          resolve();
        }
      };
      anim = requestAnimationFrame(step);
    });
  }

  const api = {
    svg,
    width,
    height,
    layer,
    pos: 0,
    /** A number labeled label.  rendering: 'flat' or 'raised', the scene's own by default. */
    number(id, label = id, opts = {}) {
      const r = opts.rendering ?? rendering;
      const mark = r === 'raised' ? v.tokenMark(layer.number, label) : v.numberMark(layer.number, label);
      return add(id, 'number', mark);
    },
    /** A line in a LINE style from mark (or point) from to mark (or point) to.  A raised line hangs as a string. */
    line(id, from, to, style = 'thin', opts = {}) {
      const r = opts.rendering ?? rendering;
      const sag = r === 'raised' && style !== 'pay' ? 7 : 0;
      return add(id, 'line', v.lineMark(layer.line, style, r), { from, to, defaults: { ...v.LINE_LOOK, sag } });
    },
    /** A frame's box: an ink outline when flat, a tray when raised. */
    box(id, opts = {}) {
      return add(id, 'box', v.boxMark(layer.box, opts.rendering ?? rendering));
    },
    tag(id, label) {
      return add(id, 'tag', v.tagMark(layer.tag, label));
    },
    /** A line of text; content is a string, or a function of the position that returns one. */
    text(id, content, { kind = 'label', anchor = 'start' } = {}) {
      return add(id, 'text', v.textMark(layer.text, kind, anchor), { content });
    },
    /** Any other mark: node goes in layer, and apply(node, look, pos) draws it from its look. */
    custom(id, node, apply, { defaults = { opacity: 1 }, layer: name = 'box' } = {}) {
      layer[name].append(node);
      return add(id, 'custom', node, { apply, defaults });
    },
    /** The node of a mark: a number's group, a line's group, a box's group. */
    node(id) {
      const m = marks.get(id);
      return m?.mark.g ?? m?.mark.text ?? m?.mark;
    },
    /** Set the states, a list of { id: look }. */
    states(list) {
      states = list;
      resolve();
      return api;
    },
    get count() {
      return states.length;
    },
    /** The full look of every mark in state k, as a Map. */
    looksOf(k) {
      return new Map([...looks[k]].map(([id, e]) => [id, e.look]));
    },
    /** Draw position pos without telling the listeners. */
    render(pos) {
      api.pos = pos;
      draw(lookAt(pos), pos);
      return api;
    },
    /** Show position pos: a state, or a point between two. */
    show(pos) {
      stop();
      const last = states.length - 1;
      const p = Math.max(0, Math.min(last, pos));
      api.render(p);
      emit(p, Math.abs(p - Math.round(p)) < 1e-9);
      return api;
    },
    /** Move to state k from wherever the scene is.  Within one step it follows the step's staging, so that a step
     *  back plays the step backward; a longer jump moves every mark at once.  Settles when it arrives. */
    go(k, { duration } = {}) {
      const last = states.length - 1;
      k = Math.min(last, Math.max(0, Math.round(k)));
      const p0 = api.pos;
      if (v.reducedMotion() || Math.abs(k - p0) < 1e-9) {
        api.show(k);
        return Promise.resolve();
      }
      const lo = Math.min(p0, k);
      const hi = Math.max(p0, k);
      const oneStep = hi - Math.floor(lo + 1e-9) <= 1 + 1e-9;
      if (oneStep) {
        const into = Math.min(last, Math.floor(lo + 1e-9) + 1);
        const ms = (duration ?? durations[into]) * (hi - lo);
        return run(ms, (t) => {
          const p = p0 + (k - p0) * t;
          api.pos = p;
          draw(lookAt(p), p);
          emit(t < 1 ? p : k, t >= 1);
        });
      }
      const from = current ?? lookAt(p0);
      const to = api.looksOf(k);
      const ms = duration ?? MOTION.jump(Math.abs(k - p0));
      return run(ms, (t) => {
        const p = t < 1 ? p0 + (k - p0) * t : k;
        api.pos = p;
        draw(t < 1 ? between(from, to, t) : to, p);
        emit(p, t >= 1);
      });
    },
    /** Move from wherever the scene is to the looks given, { id: look }, for a figure whose next state depends on
     *  the reader's move.  A mark left out is hidden where it stands; a look's at gives its window, and its through
     *  a place to pass through. */
    morph(given, { duration = 800 } = {}) {
      const from = current ?? new Map([...marks].map(([id, m]) => [id, { ...m.defaults, opacity: 0 }]));
      const to = new Map();
      const windows = {};
      const throughs = {};
      for (const [id, m] of marks) {
        const spec = given[id];
        if (spec) {
          to.set(id, { ...m.defaults, ...strip(spec) });
          if (spec.at) windows[id] = spec.at;
          if (spec.through) throughs[id] = spec.through;
        } else to.set(id, { ...(from.get(id) ?? m.defaults), opacity: 0, hop: 0 });
      }
      if (v.reducedMotion() || duration === 0) {
        stop();
        draw(to, api.pos);
        return Promise.resolve();
      }
      return run(duration, (t) => draw(t < 1 ? between(from, to, t, windows, throughs) : to, api.pos));
    },
    /** Stop a running move where it is. */
    stop,
    /** Name a number, as when the prose names it: a halo of ink on a flat mark, a lift on a raised one. */
    highlight(id, on) {
      const node = api.node(Number(id)) ?? api.node(id);
      if (node) node.classList.toggle('is-named', on);
    },
    /** Call fn(pos, settled) whenever the scene moves. */
    onMove(fn) {
      listeners.push(fn);
      return api;
    },
  };
  return api;
}

// ---------------------------------------------------------------------------------------------------------------
// The time slider
// ---------------------------------------------------------------------------------------------------------------

const ICON = {
  prev: '<path d="M10.5 3 5.5 8l5 5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>',
  next: '<path d="m5.5 3 5 5-5 5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>',
  play: '<path d="M5 3.2v9.6L12.6 8z" fill="currentColor"/>',
  pause: '<path d="M4.5 3h2.6v10H4.5zM8.9 3h2.6v10H8.9z" fill="currentColor"/>',
};
const icon = (name) => `<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">${ICON[name]}</svg>`;

/** A time slider for a scene: a track with a named stop for each state the reader should know, buttons for the state
 *  before and after and for playing on, and the arrow keys stepping from state to state.
 *    stops   [{ at, name, until }]: a named stop at state at; with until, a bracket over the states at to until,
 *            whose states between get small ticks (the moves of a game)
 *    places  where each state sits on the track, so a run of small steps can share a short stretch (default: evenly)
 *    param   the URL parameter that opens the figure at a state, read by fromParam(value) and written by toParam(k)
 *    describe(k), the words for state k; title(k), called when the scene settles at state k
 *  Returns { go(k), range }. */
export function timeline(s, host, { stops, places, param, fromParam, toParam, describe, title, label = 'Figure state' }) {
  const last = s.count - 1;
  places = places ?? Array.from({ length: s.count }, (_, k) => k);
  const end = places[last];
  const toTrack = (p) => {
    const i = Math.min(last - 1, Math.max(0, Math.floor(p)));
    return last === 0 ? 0 : places[i] + (places[i + 1] - places[i]) * (p - i);
  };
  const fromTrack = (u) => {
    let i = 0;
    while (i < last - 1 && u > places[i + 1]) i++;
    return i + (u - places[i]) / (places[i + 1] - places[i]);
  };
  host.classList.add('timeline');
  host.innerHTML = `
    <div class="tl-row">
      <button type="button" class="tl-btn tl-prev" aria-label="The state before">${icon('prev')}</button>
      <div class="tl-track">
        <input type="range" min="0" max="${end}" step="0.001" value="0" aria-label="${label}">
        <div class="tl-marks" aria-hidden="true"></div>
      </div>
      <button type="button" class="tl-btn tl-next" aria-label="The next state">${icon('next')}</button>
      <button type="button" class="tl-btn tl-play" aria-label="Play from here">${icon('play')}</button>
    </div>
    <p class="sr-only" aria-live="polite"></p>`;
  const range = host.querySelector('input');
  const marksBox = host.querySelector('.tl-marks');
  const live = host.querySelector('[aria-live]');
  const playButton = host.querySelector('.tl-play');
  const pct = (k) => `${(100 * places[k]) / end}%`;
  const stopNodes = [];
  for (const m of stops) {
    const tick = document.createElement('span');
    tick.className = m.until !== undefined ? 'tl-mark tl-span' : 'tl-mark';
    tick.style.left = pct(m.at);
    if (m.until !== undefined) {
      tick.style.width = `calc(${pct(m.until)} - ${pct(m.at)})`;
      for (let k = m.at + 1; k < m.until; k++) {
        const minor = document.createElement('span');
        minor.className = 'tl-minor';
        minor.style.left = `calc(100% * ${(places[k] - places[m.at]) / (places[m.until] - places[m.at])})`;
        tick.appendChild(minor);
      }
    }
    const b = document.createElement('button');
    b.type = 'button';
    b.tabIndex = -1;
    b.textContent = m.name;
    b.addEventListener('click', () => {
      setPlaying(false);
      s.go(m.at);
    });
    tick.appendChild(b);
    marksBox.appendChild(tick);
    stopNodes.push(tick);
  }
  // On a narrow track the names take turns above and below it, so that none runs into the next.
  const fit = () => {
    host.classList.remove('is-staggered');
    const labels = stopNodes.map((n) => n.querySelector('button').getBoundingClientRect());
    const crowded = labels.some((r, i) => i > 0 && r.left < labels[i - 1].right + 4);
    host.classList.toggle('is-staggered', crowded);
  };
  if (typeof ResizeObserver !== 'undefined') new ResizeObserver(fit).observe(host);

  let playing = false;
  function setPlaying(on) {
    playing = on;
    playButton.innerHTML = icon(on ? 'pause' : 'play');
    playButton.setAttribute('aria-label', on ? 'Pause' : 'Play from here');
  }
  const settledIndex = () => Math.round(s.pos);
  s.onMove((p, settled) => {
    range.value = toTrack(p);
    host.style.setProperty('--progress', `${(100 * toTrack(p)) / end}%`);
    if (!settled) return;
    const k = Math.round(p);
    range.setAttribute('aria-valuetext', describe ? describe(k) : `state ${k + 1}`);
    if (title) title(k);
    stops.forEach((m, i) => {
      const on = m.until !== undefined ? k >= m.at && k <= m.until : k === m.at;
      stopNodes[i].classList.toggle('is-on', on);
    });
    live.textContent = describe ? describe(k) : '';
    if (param && toParam) {
      const url = new URL(location.href);
      url.searchParams.set(param, toParam(k));
      history.replaceState(null, '', url);
    }
  });
  range.addEventListener('input', () => {
    setPlaying(false);
    s.show(fromTrack(Number(range.value)));
  });
  range.addEventListener('change', () => s.go(fromTrack(Number(range.value))));
  range.addEventListener('keydown', (e) => {
    const k = settledIndex();
    const to = { ArrowRight: k + 1, ArrowUp: k + 1, ArrowLeft: k - 1, ArrowDown: k - 1, Home: 0, End: last, PageUp: k + 3, PageDown: k - 3 }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    setPlaying(false);
    s.go(to);
  });
  host.querySelector('.tl-prev').addEventListener('click', () => {
    setPlaying(false);
    s.go(Math.ceil(s.pos - 1e-6) - 1);
  });
  host.querySelector('.tl-next').addEventListener('click', () => {
    setPlaying(false);
    s.go(Math.floor(s.pos + 1e-6) + 1);
  });
  playButton.addEventListener('click', async () => {
    if (playing) {
      setPlaying(false);
      s.stop();
      return;
    }
    setPlaying(true);
    if (settledIndex() >= last) s.show(0);
    while (playing && Math.round(s.pos) < last) {
      await s.go(Math.floor(s.pos + 1e-6) + 1);
      if (!playing) break;
      await new Promise((r) => setTimeout(r, MOTION.pause));
    }
    setPlaying(false);
  });
  // Open where the URL says, or at the first state.
  let start = 0;
  if (param) {
    const value = new URLSearchParams(location.search).get(param);
    if (value !== null) {
      const k = fromParam(value);
      if (Number.isInteger(k) && k >= 0 && k <= last) start = k;
    }
  }
  s.show(start);
  requestAnimationFrame(fit);
  return { go: (k) => s.go(k), range, setPlaying };
}
