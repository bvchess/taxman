// The specimens of the style page: the scale of the type, the color tokens and their tones, every mark in both renderings, a
// promotion in motion, the controls on a small live figure, the layout at each width, and the motion itself.  Each
// draws with the site's own modules, so the page shows what the site does, not a picture of it.

import { MOTION, SHAPE, el, fmt, numberSVG } from '../js/vocab.js';
import { contrast, separation } from '../js/cvd.js';
import { loadSmall } from '../js/data.js';
import * as M from '../js/model.js';
import { scene, timeline } from '../js/scene.js';
import { ENTRIES } from './legend.js';

// ---------------------------------------------------------------------------------------------------------------
// Type
// ---------------------------------------------------------------------------------------------------------------

/** The scale of the text, measured from the page itself, so the numbers are the ones the stylesheet gives. */
export function mountScale(host) {
  const rows = [
    ['h1', 'The title of a page', 'h1', 'One a page.'],
    ['h2', 'A chapter', 'h2', 'Each chapter, with its number drawn flat beside it.'],
    ['h3', 'A section of a chapter', 'h3', 'The sections inside a chapter.'],
    ['p.lede', 'The lede under a title', 'p', 'The sentence or two that say what a page is for.'],
    ['p', 'The text of the story, a line of at most 36 rem.', 'p', 'Everything the story says.'],
    ['p.scale-note', 'A note of the technical layer', 'p', 'The right column: cards, notes and links.'],
    ['p.scale-caption', 'The caption under a figure', 'p', 'What a figure shows in its present state.'],
  ];
  host.innerHTML = rows.map(([sel, text, tag, use]) => {
    const cls = sel.includes('.') ? sel.split('.')[1] : '';
    return `<div class="scale-row"><div class="scale-spec" data-spec></div><${tag} class="${cls} scale-sample">${text}</${tag}><div class="scale-use">${use}</div></div>`;
  }).join('');
  const fill = () => {
    for (const row of host.querySelectorAll('.scale-row')) {
      const sample = row.querySelector('.scale-sample');
      const cs = getComputedStyle(sample);
      row.querySelector('[data-spec]').textContent = `${Math.round(parseFloat(cs.fontSize))} px, ${cs.fontWeight}, line ${(parseFloat(cs.lineHeight) / parseFloat(cs.fontSize)).toFixed(2)}`;
    }
  };
  fill();
  document.fonts?.ready.then(fill);
  addEventListener('resize', fill);
}

/** Numbers in a figure: the same numbers flat and raised, at the sizes the figures use. */
export function mountFigureNumbers(host) {
  const nums = [1, 7, 10, 17, 71, 101, 144];
  for (const r of ['flat', 'raised']) {
    const row = document.createElement('div');
    row.className = 'num-row';
    row.innerHTML = `<span class="num-row-name">${r}</span>`;
    for (const n of nums) row.appendChild(numberSVG(n, { rendering: r, looks: [n > 10 ? 'move' : 'factor'], size: 46, pad: 3 }));
    host.appendChild(row);
  }
}

// ---------------------------------------------------------------------------------------------------------------
// Color
// ---------------------------------------------------------------------------------------------------------------

/** The color tokens, by group, with what each means.  The values are read from the stylesheet, their one home. */
export const TOKENS = [
  ['Ink and paper', [
    ['--paper', 'The page, and the face of every mark.'],
    ['--ink', 'Text, the outline of a flat mark, a payment, the reader’s place.'],
    ['--ink-2', 'Secondary text: ledes, captions, notes.'],
    ['--ink-3', 'Quiet text: labels, kickers, the names of a slider’s stops.'],
    ['--mid', 'A thin line in a flat figure; a dotted line or arrow.'],
    ['--faint', 'A faded mark or line, out of focus.'],
    ['--hair', 'Hairlines: rules, the margin’s line, grids.'],
    ['--wash', 'The faintest ground: a folded note, a chapter not yet written, a hover.'],
  ]],
  ['The two renderings', [
    ['--kept', 'Flat: the gray fill of a kept number.'],
    ['--kept-face', 'Raised: the face of a kept token.'],
    ['--kept-rim', 'Raised: the rim of a kept token.'],
    ['--rim', 'Raised: the rim of a token.'],
    ['--side', 'Raised: the edge seen below a token’s face.'],
    ['--string', 'Raised: a string, a thin line hung slack.'],
    ['--tray', 'Raised: a tray, the ground of the pot and of a frame.'],
    ['--tray-edge', 'Raised: a tray’s edge, a button’s edge.'],
  ]],
  ['The player and the taxman, the only colors', [
    ['--player', 'A mark in the player’s color: an outline, the player’s share in a chart, a dot.  Nothing else is blue.'],
    ['--player-face', 'The face of a raised token the player took, under an ink number.'],
    ['--player-rim', 'The rim of that token, in a mid shade; --player-side, its edge.'],
    ['--player-tint', 'The player’s tray, and the fill of a flat move taken; --player-edge, the tray’s edge.'],
    ['--player-ink', 'Words in the player’s color; ink in the light tones, where blue would not reach 4.5:1.'],
    ['--taxman', 'A mark in the taxman’s color: his tax and the rest.  Nothing else is orange.'],
    ['--taxman-face', 'The face of a raised token the taxman took, under an ink number.'],
    ['--taxman-rim', 'The rim of that token; --taxman-side, its edge.'],
    ['--taxman-tint', 'The taxman’s tray, and the fill of a flat number he took; --taxman-edge, the tray’s edge.'],
    ['--taxman-ink', 'Words in the taxman’s color; ink in the light tones.'],
  ]],
];

/** The three tones of the player's and the taxman's colors, the site's own last.  Each is a set of tokens in
 *  css/site.css; data-tone sets one inside an element, and ?tone= sets one for a whole page. */
export const TONES = [
  { key: 'deep', name: 'Deep', role: 'the first tone, for reference: full color under white numbers' },
  { key: 'light', name: 'Light', role: 'lighter: light faces under ink, outlines in a mid shade' },
  { key: 'lightest', name: 'Lightest', role: 'the site’s tone: lighter still; the outlines stay at 3:1 on the page' },
];

/** The pair at each tone, side by side: the trays at the end of an optimal game of 10, from data/small.json,
 *  a flat move and its tax, a chart's lines, and the pair's separation for color-blind readers, worked out from the
 *  tokens themselves. */
export async function mountTones(host) {
  const site = document.documentElement.dataset.tone ?? 'lightest';
  const n = 10;
  const game = (await loadSmall()).find((g) => g.n === n);
  const end = M.replay(n, game.moves);
  const sum = (xs) => xs.reduce((a, b) => a + b, 0);
  const shapeOf = (x) => (x > n / 2 ? 'move' : 'factor');
  host.innerHTML = TONES.map((t) => `
    <figure class="tone" data-tone="${t.key}">
      <figcaption><b>${t.name}</b>${t.key === site ? '<span class="tone-here">this page</span>' : ''}<span>${t.role}</span></figcaption>
      <div class="trays">
        <div class="tray-box player"><header><span class="who">You</span><span class="score">${sum(end.player)}</span></header><div class="slots" data-who="player"></div></div>
        <div class="tray-box taxman"><header><span class="who">Taxman</span><span class="score">${sum(end.taxman)}</span></header><div class="slots" data-who="taxman"></div></div>
      </div>
      <div class="tone-flat"></div>
      <div class="tone-values"></div>
    </figure>`).join('');
  for (const fig of host.querySelectorAll('.tone')) {
    for (const who of ['player', 'taxman']) {
      const slots = fig.querySelector(`[data-who="${who}"]`);
      for (const x of end[who]) slots.appendChild(numberSVG(x, { rendering: 'raised', looks: [who === 'player' ? 'move' : shapeOf(x), who], size: 40, pad: 3 }));
    }
    // A flat move and its tax, the rest the taxman took, and two lines of a chart, each named in ink.
    const svg = el('svg', { viewBox: '0 0 300 76', width: '100%', role: 'img', 'aria-label': 'A flat move, 9, the player’s, paid for by its tax, 3, the taxman’s; 17, the rest, the taxman’s; and a chart line in each color.' }, fig.querySelector('.tone-flat'));
    const s = scene(svg, { width: 300, height: 76 });
    s.number(9, 9).number(3, 3).number(17, 17).line('pay', 9, 3, 'pay');
    s.states([{ 9: { x: 22, y: 20, shape: SHAPE.circle, player: 1 }, 3: { x: 22, y: 60, shape: SHAPE.square, taxman: 1 }, 17: { x: 64, y: 40, shape: SHAPE.circle, dashed: 1, taxman: 1 }, pay: {} }]).render(0);
    const line = (d, tone, label, y) => {
      el('path', { d, fill: 'none', 'stroke-width': 2, style: `stroke: var(${tone})` }, svg);
      const t = el('text', { x: 296, y, 'text-anchor': 'end', class: 'v-text' }, svg);
      t.textContent = label;
    };
    line('M100 22 C130 8 150 30 180 18 S230 14 250 16', '--player', 'player', 20);
    line('M100 58 C130 66 150 46 180 56 S230 60 250 58', '--taxman', 'taxman', 60);
    const cs = getComputedStyle(fig);
    const hex = (name) => cs.getPropertyValue(name).trim();
    const marks = [hex('--player'), hex('--taxman')];
    const faces = [hex('--player-face'), hex('--taxman-face')];
    const sep = ([a, b]) => `deutan ${separation(a, b, 'deutan').toFixed(1)}, protan ${separation(a, b, 'protan').toFixed(1)}`;
    fig.querySelector('.tone-values').innerHTML = `
      <p><span class="chip-swatch" style="background:${marks[0]}"></span><span class="chip-swatch" style="background:${marks[1]}"></span>marks ${marks.join(' ')}: ${sep(marks)}; on the page ${contrast(marks[0], '#ffffff').toFixed(1)}:1 and ${contrast(marks[1], '#ffffff').toFixed(1)}:1</p>
      <p><span class="chip-swatch" style="background:${faces[0]}"></span><span class="chip-swatch" style="background:${faces[1]}"></span>faces ${faces.join(' ')}: ${sep(faces)}</p>`;
  }
}

export function mountTokens(host) {
  const cs = getComputedStyle(document.documentElement);
  host.innerHTML = TOKENS.map(([group, list]) => `<div class="token-group"><h4>${group}</h4><ul>${list.map(([name, words]) => {
    const value = cs.getPropertyValue(name).trim();
    return `<li><span class="swatch" style="background:var(${name})"></span><span class="token-name">${name}<span class="token-value">${value}</span></span><span class="token-words">${words}</span></li>`;
  }).join('')}</ul></div>`).join('');
}

// ---------------------------------------------------------------------------------------------------------------
// The marks
// ---------------------------------------------------------------------------------------------------------------

/** Every entry of the legend, in both renderings, with what it means and when to use it. */
export function mountMarks(host) {
  host.innerHTML = '<div class="marks-head" aria-hidden="true"><span>flat</span><span>raised</span><span>mark</span><span>when to use it</span></div>';
  for (const entry of ENTRIES) {
    const row = document.createElement('div');
    row.className = 'marks-row';
    for (const r of ['flat', 'raised']) {
      const cell = document.createElement('span');
      cell.className = 'marks-pic';
      const pic = entry.draw(r);
      if (pic) {
        pic.setAttribute('width', 64);
        pic.setAttribute('height', 54);
        cell.appendChild(pic);
      } else cell.innerHTML = `<span class="marks-none">${r === 'flat' ? 'raised' : 'flat'} only</span>`;
      row.appendChild(cell);
    }
    row.insertAdjacentHTML('beforeend', `<span class="marks-name"><b>${entry.name}</b>${entry.words}</span><span class="marks-use">${entry.use}</span>`);
    host.appendChild(row);
  }
}

// A promotion, small: two frames side by side, 9 a candidate among the factors of the frame behind, then a move of
// the frame in front, paid for by 3.  18 is joined to its maximal factors 9 and 6; while 9 is a candidate, a dotted
// line joins it to the 3 that would pay for it, and it gives way to the hop.  The same states drawn in either
// rendering.
function promotionScene(svg, rendering) {
  const s = scene(svg, { width: 260, height: 132, rendering, label: `A promotion, ${rendering}: 9 leaves the factors of one frame and becomes a move of the frame in front, paid for by 3.` });
  s.box('behind').box('front');
  s.number('ghost', 9).number(9, 9).number(6, 6).number(18, 18).number(3, 3).number(15, 15).number(5, 5);
  s.line('18-9', 18, 9, 'thin').line('18-6', 18, 6, 'thin').line('15-3', 15, 3, 'thin').line('15-5', 15, 5, 'thin');
  s.line('rise', 'ghost', 9, 'hop').line('link', 9, 3, 'link').line('pay', 9, 3, 'pay');
  const base = {
    behind: { x: 8, y: 10, w: 100, h: 112 },
    front: { x: 128, y: 10, w: 124, h: 112 },
    18: { x: 36, y: 66, shape: SHAPE.circle, kept: 1 },
    3: { x: 220, y: 44, shape: SHAPE.square },
    5: { x: 220, y: 92, shape: SHAPE.square },
    15: { x: 158, y: 92, shape: SHAPE.circle, kept: 1 },
    6: { x: 82, y: 88, shape: SHAPE.square },
    '18-6': {}, '15-3': {}, '15-5': {},
  };
  return { s, base };
}

function promotionStates(base) {
  return [
    { ...base, 9: { x: 82, y: 44, shape: SHAPE.square }, '18-9': {} },
    { ...base, 9: { x: 82, y: 44, shape: SHAPE.square, ring: 1 }, '18-9': {}, link: {} },
    {
      ...base, _duration: 1500,
      ghost: { x: 82, y: 44, shape: SHAPE.square, dashed: 1, faded: 1, depth: 0, at: [0.1, 0.5] },
      9: { x: 158, y: 44, shape: SHAPE.circle, kept: 1, ring: 1, at: [0.05, 0.8] },
      rise: { at: [0.45, 0.9] }, pay: { at: [0.6, 1] }, '18-9': { opacity: 0, at: [0, 0.3] },
      link: { opacity: 0, at: [0, 0.25] },
    },
    {
      ...base,
      ghost: { x: 82, y: 44, shape: SHAPE.square, dashed: 1, faded: 1, depth: 0 },
      9: { x: 158, y: 44, shape: SHAPE.circle, player: 1 },
      3: { ...base[3], taxman: 1 }, rise: { faded: 1 }, pay: {},
    },
  ];
}

/** The promotion in both renderings, side by side, with a button that plays it again. */
export function mountPromotion(host) {
  const pairs = [...host.querySelectorAll('svg')].map((svg) => promotionScene(svg, svg.dataset.rendering));
  for (const { s, base } of pairs) s.states(promotionStates(base).slice(1, 3)).show(0);
  const replay = async () => {
    for (const { s } of pairs) s.show(0);
    await new Promise((r) => setTimeout(r, 350));
    await Promise.all(pairs.map(({ s }) => s.go(1)));
  };
  host.querySelector('[data-act="replay"]').addEventListener('click', replay);
  // Play once when the specimen first comes into view.
  const seen = new IntersectionObserver((entries) => {
    if (entries.some((e) => e.isIntersecting)) {
      seen.disconnect();
      replay();
    }
  }, { threshold: 0.6 });
  seen.observe(host);
}

/** The controls, live: the same promotion as a stepped figure, with a slider of named stops, the step and play
 *  buttons, the arrow keys, and words in the prose that move it.  ?demo=k opens it at state k. */
export function mountControls(host) {
  const { s, base } = promotionScene(host.querySelector('svg'), 'flat');
  s.states(promotionStates(base));
  const names = ['factor', 'candidate', 'promote', 'play'];
  const words = [
    '9 is a factor of the frame behind, which can spare it.',
    '9 is a candidate: the frame in front can take it on.',
    '9 rounds into a move of the frame in front, and 3 pays for it.',
    'Played: 9 is the player’s, and 3, its payment, the taxman’s.',
  ];
  const caption = host.querySelector('.fig-caption');
  timeline(s, host.querySelector('.fig-timeline'), {
    stops: names.map((name, at) => ({ at, name })),
    param: 'demo',
    fromParam: (v) => Number(v) - 1,
    toParam: (k) => String(k + 1),
    describe: (k) => words[k],
    title: (k) => {
      caption.textContent = words[k];
    },
    label: 'The small figure: state',
  });
  return {
    goState: (k) => s.go(Number(k) - 1),
    stopName: (k) => names[Number(k) - 1],
    highlight: (n, on) => s.highlight(n, on),
  };
}

// ---------------------------------------------------------------------------------------------------------------
// The layout at each width
// ---------------------------------------------------------------------------------------------------------------

/** The three columns and how they fold, drawn small: a box for each column at 1440, 1100, 820 and 390 px. */
export function mountLayout(host) {
  const widths = [
    { w: 1440, nav: 'rail', tech: 'margin', words: 'Three columns: the contents, the story, the technical layer.' },
    { w: 1100, nav: 'bar', tech: 'margin', words: 'Below 1300 px the contents fold into a bar with a button.' },
    { w: 820, nav: 'bar', tech: 'folded', words: 'Below about 1050 px each note folds under its paragraph, and opens when tapped.' },
    { w: 390, nav: 'bar', tech: 'folded', words: 'A phone: the same, with a 16 px gutter; figures scale to fit.' },
  ];
  const scale = 0.16;
  host.innerHTML = widths.map(({ w, nav, tech, words }) => {
    const W = Math.round(w * scale);
    const H = 150;
    const parts = [];
    const pad = Math.max(3, Math.round((w < 600 ? 16 : 36) * scale));
    let x = 0;
    let top = 4;
    if (nav === 'rail') {
      parts.push(`<rect x="2" y="4" width="${Math.round(224 * scale)}" height="${H - 8}" class="lay-nav"/><text x="${2 + (224 * scale) / 2}" y="22" class="lay-t" text-anchor="middle">nav</text>`);
      x = 224 * scale;
    } else {
      parts.push(`<rect x="2" y="4" width="${W - 4}" height="12" class="lay-nav"/><text x="${W / 2}" y="13.5" class="lay-t" text-anchor="middle">contents bar</text>`);
      top = 22;
    }
    const avail = W - x - 2 * pad;
    const story = Math.min(avail, (tech === 'margin' ? 648 : 648) * scale);
    const margin = tech === 'margin' ? 297 * scale : 0;
    const gutter = tech === 'margin' ? 43 * scale : 0;
    const total = story + gutter + margin;
    const sx = x + Math.max(pad, (W - x - total) / 2);
    parts.push(`<rect x="${sx}" y="${top}" width="${story}" height="${H - top - 4}" class="lay-story"/><text x="${sx + story / 2}" y="${top + 18}" class="lay-t" text-anchor="middle">story</text>`);
    if (tech === 'margin') {
      const mx = sx + story + gutter;
      parts.push(`<rect x="${mx}" y="${top}" width="${margin}" height="${H - top - 4}" class="lay-tech"/><text x="${mx + margin / 2}" y="${top + 18}" class="lay-t" text-anchor="middle">technical</text>`);
    } else {
      for (const y of [top + 46, top + 92]) parts.push(`<rect x="${sx + 3}" y="${y}" width="${story - 6}" height="9" class="lay-fold"/>`);
      parts.push(`<text x="${sx + story / 2}" y="${top + 72}" class="lay-t" text-anchor="middle">notes folded</text>`);
    }
    return `<figure class="layout-at"><svg viewBox="0 0 ${W} ${H}" width="${W}" height="${H}" role="img" aria-label="${w} px: ${words}"><rect x="0.5" y="0.5" width="${W - 1}" height="${H - 1}" rx="6" class="lay-page"/>${parts.join('')}</svg><figcaption><b>${fmt(w)} px</b>${words}</figcaption></figure>`;
  }).join('');
}

// ---------------------------------------------------------------------------------------------------------------
// Motion
// ---------------------------------------------------------------------------------------------------------------

/** The easing curves, drawn: the share of the way a mark has gone (up) against the share of the time (across). */
export function mountCurves(host) {
  const curves = [['glide', MOTION.glide, 'flat marks'], ['settle', MOTION.settle, 'raised tokens set down'], ['smooth', MOTION.smooth, 'the look of a raised token']];
  host.innerHTML = '';
  for (const [name, fn, use] of curves) {
    const svg = el('svg', { viewBox: '-6 -14 112 128', width: 112, height: 128, role: 'img', 'aria-label': `The curve ${name}` });
    el('rect', { x: 0, y: 0, width: 100, height: 100, fill: 'none', style: 'stroke: var(--hair)' }, svg);
    let d = '';
    for (let i = 0; i <= 60; i++) {
      const t = i / 60;
      d += `${i ? 'L' : 'M'}${(t * 100).toFixed(1)} ${(100 - fn(t) * 100).toFixed(1)}`;
    }
    el('path', { d, fill: 'none', 'stroke-width': 2.2, style: 'stroke: var(--ink)' }, svg);
    const fig = document.createElement('figure');
    fig.className = 'curve';
    fig.append(svg);
    fig.insertAdjacentHTML('beforeend', `<figcaption><b>${name}</b>${use}</figcaption>`);
    host.append(fig);
  }
}

/** Three lanes of motion: a flat mark gliding, a raised token carried and set down, and a raised token turning over. */
export function mountMotion(host) {
  const lanes = [
    { r: 'flat', from: { shape: SHAPE.circle, kept: 1 }, to: { shape: SHAPE.circle, kept: 1 } },
    { r: 'raised', from: { shape: SHAPE.circle }, to: { shape: SHAPE.circle, player: 1 } },
    { r: 'raised', from: { shape: SHAPE.square }, to: { shape: SHAPE.circle, kept: 1 } },
  ];
  const scenes = [...host.querySelectorAll('svg')].map((svg, i) => {
    const lane = lanes[i];
    const s = scene(svg, { width: 300, height: 52, rendering: lane.r, label: svg.getAttribute('aria-label') });
    s.line('track', { x: 30, y: 46 }, { x: 270, y: 46 }, 'rule');
    s.number('m', i === 2 ? 9 : 19);
    s.states([{ track: {}, m: { ...lane.from, x: 30, y: 26 } }, { track: {}, m: { ...lane.to, x: 270, y: 26 }, _duration: MOTION.step }]).show(0);
    return s;
  });
  host.querySelector('[data-act="motion"]').addEventListener('click', async () => {
    for (const s of scenes) s.show(0);
    await new Promise((r) => setTimeout(r, 250));
    await Promise.all(scenes.map((s) => s.go(1)));
  });
}
