// The legend: every mark of the vocabulary, drawn by vocab.js itself as the figures draw it, flat, so the legend cannot
// drift from the figures.  The one exception is the coin, the raised circle a reader can press in a game they play.
// The notation page shows the marks a reader needs; the style page shows every entry, with when to use it.

import {
  BOX, LINE_LOOK, NUMBER, TAG, boxMark, el, installDefs, lineMark, look, numberMark, setBox, setLine, setNumber, setTag,
  setToken, tagMark, token,
} from '../js/vocab.js';

const W = 60;
const H = 50;
const WIDE = 76; // the box of a sample whose line needs room to show

function svgBox(w = W, h = H) {
  installDefs();
  return el('svg', { viewBox: `0 0 ${w} ${h}`, width: w * 0.8, height: h * 0.8, 'aria-hidden': 'true', class: 'v-figure' });
}

/** Draw the number label with the look parts at (x, y) in svg, in the rendering given; returns its full look. */
export function put(svg, rendering, label, x, y, ...parts) {
  const p = { ...NUMBER, ...look(...parts), x, y };
  if (rendering === 'raised') {
    const t = token(label);
    svg.appendChild(t);
    setToken(t, p);
  } else setNumber(numberMark(svg, label), p);
  return p;
}

function joined(svg, rendering, style, a, b) {
  const m = lineMark(svg, style, rendering);
  svg.insertBefore(m.g, svg.firstChild);
  setLine(m, a, b, { ...LINE_LOOK, sag: rendering === 'raised' && style !== 'pay' ? 6 : 0 });
}

const one = (label, ...parts) => (r) => {
  const svg = svgBox();
  put(svg, r, label, W / 2, H / 2 - 1, ...parts);
  return svg;
};

const pair = (style, [a, ...pa], [b, ...pb]) => (r) => {
  const svg = svgBox(WIDE);
  const p = put(svg, r, a, 13, 12, ...pa, { scale: 0.66 });
  const q = put(svg, r, b, WIDE - 13, 38, ...pb, { scale: 0.66 });
  joined(svg, r, style, p, q);
  return svg;
};

function frameSample(closed) {
  return (r) => {
    const svg = svgBox();
    setBox(boxMark(svg, r), { ...BOX, x: 6, y: 8, w: 48, h: 34, dashed: closed ? 1 : 0 });
    return svg;
  };
}

function hopSample(r) {
  const svg = svgBox(WIDE);
  // A candidate leaves its square for the frame in front and ends there as a gray double circle; the line runs from
  // the edge of one to the edge of the other, so the marks stand far enough apart for the dots and the head to show.
  const from = put(svg, r, 9, 13, 38, 'factor', 'candidate', { scale: 0.66 });
  const to = put(svg, r, 9, WIDE - 13, 12, 'move', 'kept', 'candidate', { scale: 0.66 });
  joined(svg, r, 'hop', from, to);
  return svg;
}

function ruleSample(r) {
  const svg = svgBox();
  const m = lineMark(svg, 'rule', r);
  setLine(m, { x: 8, y: H / 2, scale: 0, shape: 1, ring: 0 }, { x: W - 8, y: H / 2, scale: 0, shape: 1, ring: 0 }, LINE_LOOK);
  return svg;
}

function tagSample(r) {
  const svg = svgBox();
  put(svg, r, 19, 28, 28, 'move', 'player');
  setTag(tagMark(svg, 1), { ...TAG, x: 13, y: 11 });
  return svg;
}

function columnsSample(r) {
  const svg = svgBox();
  const left = put(svg, r, 6, 15, 20, 'factor', { scale: 0.6 });
  const right = put(svg, r, 3, 45, 20, 'factor', { scale: 0.6 });
  joined(svg, r, 'thin', left, right);
  for (const [x, k] of [[13, 2], [47, 1]]) {
    const t = el('text', { x, y: 44, 'text-anchor': 'middle', class: 'v-text v-text-name' }, svg);
    t.style.fontSize = '8px';
    t.textContent = `layer ${k}`;
  }
  return svg;
}

/** Every mark, in the key's order: how to draw it (flat, or raised when raised is set) and when to use it.  Its name
 *  and meaning are the notation page's own words, in the item of notation.html with the same data-mark. */
export const ENTRIES = [
  {
    key: 'move',
    use: 'A number can be taken only while one of its divisors is still in the pot. Once none is left it can ' +
      'never be a move, and its circle turns into a square.',
    draw: one(21, 'move'),
  },
  {
    key: 'factor',
    use: 'A number that is not a move: it goes to the taxman as tax or at the end. 1 has no divisor, so it is ' +
      'always a square.',
    draw: one(7, 'factor'),
  },
  {
    key: 'kept',
    use: 'A number a figure has put in the set of moves, until the player plays it.', draw: one(19, 'move', 'kept'),
  },
  {
    key: 'candidate',
    use: 'On a square. A candidate goes to the taxman unless it is promoted: its frame can spare it and the frame in ' +
      'front can take it on.',
    draw: one(9, 'factor', 'candidate'),
  },
  {
    key: 'promoted',
    use: 'A candidate keeps its double outline when it becomes a circle, so a double circle is always a promotion. ' +
      'It is gray until played and blue once played.',
    draw: one(12, 'move', 'kept', 'candidate'),
  },
  {
    key: 'player',
    use: 'Only for the player and the player\u2019s moves, named by a label, a tray or the caption.',
    draw: one(9, 'move', 'player'),
  },
  {
    key: 'taxman',
    use: 'Only for the taxman and what he takes, named by a label, a tray or the caption. A source no one ' +
      'played, which he takes at the end, is an orange square.',
    draw: one(3, 'factor', 'taxman'),
  },
  {
    key: 'thin',
    use: 'Joins a number to each maximal factor, running rightward. Between the columns of a graph it leaves and ' +
      'arrives level, and crosses no number.',
    draw: pair('thin', [12, 'move'], [6, 'factor']),
  },
  {
    key: 'pay',
    use: 'From a move to the factor it takes as its payment. In a game laid out in rows, it runs down the ' +
      'move\u2019s stack of tax.',
    draw: pair('pay', [18, 'move', 'kept'], [6, 'factor']),
  },
  {
    key: 'link',
    use: 'From each candidate to each of its maximal factors in the frame in front, once the story names the ' +
      'candidates. It has no arrowhead.',
    draw: pair('link', [9, 'factor', 'candidate'], [3, 'factor']),
  },
  {
    key: 'hop',
    use: 'The only arrow the figures draw. It runs from where the candidate was to the gray double circle it becomes, ' +
      'and is drawn only while the candidate moves.',
    draw: hopSample,
  },
  {
    key: 'rule',
    use: 'Cuts the board of a game into its top half and its bottom half.', draw: ruleSample,
  },
  {
    key: 'frame',
    use: 'The figures draw it round a frame\u2019s sources and factors. In a game you play, it is a tray.',
    draw: frameSample(false),
  },
  {
    key: 'closed',
    use: 'From the moment the story counts its freedoms.', draw: frameSample(true),
  },
  {
    key: 'columns',
    use: 'Every figure that lays a game out by layers or frames; lines run rightward, toward the front.',
    draw: columnsSample,
  },
  {
    key: 'named',
    use: 'Point at a number in the prose; a figure also uses it for what its caption names.',
    draw: one(19, 'move', 'named'),
  },
  {
    key: 'tag',
    use: 'When the player plays a game in place; a game laid out in trays fills the player\u2019s tray in ' +
      'order instead.',
    draw: tagSample,
  },
  {
    key: 'coin',
    use: 'Only in a game you play: a circle the player can take now stands up from the page. Every other number is ' +
      'drawn flat.',
    draw: one(21, 'move'), raised: true,
  },
];

/** The drawing of an entry: flat, or raised for the coin. */
export const sample = (entry) => entry.draw(entry.raised ? 'raised' : 'flat');

/** Draw each entry's sample into the legend's markup: an item with a data-mark takes the sample of that entry as its
 *  first child.  The names, the words and the groups are the page's own; every item must name an entry, and every
 *  entry must have an item, so the page and the drawings cannot drift apart. */
export function mountLegend(host) {
  const byKey = new Map(ENTRIES.map((e) => [e.key, e]));
  const seen = new Set();
  for (const item of host.querySelectorAll('[data-mark]')) {
    const key = item.dataset.mark;
    if (!byKey.has(key)) throw new Error(`the legend has an item for the mark "${key}", which has no entry`);
    seen.add(key);
    item.insertBefore(sample(byKey.get(key)), item.firstChild);
  }
  for (const key of byKey.keys()) {
    if (!seen.has(key)) throw new Error(`the mark "${key}" has an entry but no item in the legend`);
  }
}
