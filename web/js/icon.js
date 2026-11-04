// The site's icon: a small piece of a maximal-factor graph from game 10, with no numbers, laid out as
// figures/graph-game.js lays out a game (one column a layer, the highest layer at the left, the small numbers at the
// top of each column).  Six marks in three columns, joined by five lines, each from a number to a maximal factor:
// 8-4, 4-2, 6-2, 10-2 and 10-5.  A number above n/2 (6, 8, 10) is a coin, a circle; the rest (2, 4, 5) are tiles,
// rounded squares a little smaller than the coins.  6 is the one the player has taken.
//
// The picture is written here once.  iconSVG({ size }) returns the markup in the page's colors (CSS variables, so the
// tone and the dark theme apply), for the rail; iconSVG({ size, themed: false }) returns a standalone file with fixed
// light colors and a dark-scheme block that swaps the ink and the paper, for the favicon.  tools/icon.mjs writes the
// file, web/icon.svg; run it after changing anything here.

const BOX = 32;
const LINE = 1.6; // the lines are a little thinner than the outlines
const OUTLINE = 2;
const COIN = 3.5; // the radius of a coin's circle, to the middle of its outline
const TILE = 2.9; // half the side of a tile, to the middle of its outline
const CORNER = 1.5;

const COLUMN = { 8: 4.5, 4: 15.2, 6: 15.2, 10: 15.2, 2: 27.5, 5: 27.5 };
const ROW = { 8: 4.5, 4: 4.5, 2: 4.5, 6: 15.8, 10: 27.5, 5: 21.7 };
const MARKS = [8, 4, 6, 10, 2, 5];
const COINS = new Set([6, 8, 10]);
const LINES = [[8, 4], [4, 2], [6, 2], [10, 2], [10, 5]];
const PLAYER = 6;

// Each role of color: its themed style, and its fixed values in the file for the light and the dark scheme.
const ROLES = {
  line: { style: 'stroke:var(--ink)', light: { stroke: '#1e2227' }, dark: { stroke: '#e6e8eb' } },
  plain: {
    style: 'fill:var(--paper);stroke:var(--ink)',
    light: { fill: '#ffffff', stroke: '#1e2227' },
    dark: { fill: '#1e2227', stroke: '#e6e8eb' },
  },
  player: {
    style: 'fill:var(--player-face);stroke:var(--player-rim)',
    light: { fill: '#c6dcf7', stroke: '#5090de' },
    dark: { fill: '#c6dcf7', stroke: '#5090de' },
  },
};

const css = (props) => Object.entries(props).map(([k, v]) => `${k}:${v}`).join(';');
const num = (v) => String(Math.round(v * 100) / 100);

function lineMarkup([a, b], paint) {
  return `<line x1="${COLUMN[a]}" y1="${ROW[a]}" x2="${COLUMN[b]}" y2="${ROW[b]}" ${paint('line')}/>`;
}

function markMarkup(n, paint) {
  const x = COLUMN[n];
  const y = ROW[n];
  const role = n === PLAYER ? 'player' : 'plain';
  if (COINS.has(n)) return `<circle cx="${x}" cy="${y}" r="${COIN}" ${paint(role)}/>`;
  const side = num(TILE * 2);
  return `<rect x="${num(x - TILE)}" y="${num(y - TILE)}" width="${side}" height="${side}" rx="${CORNER}" ${paint(role)}/>`;
}

function fileStyle() {
  const rules = (scheme) => Object.keys(ROLES).map((r) => `.${r}{${css(ROLES[r][scheme])}}`).join('');
  return `svg{stroke-width:${OUTLINE};stroke-linecap:round}.line{stroke-width:${LINE}}${rules('light')}`
    + `@media (prefers-color-scheme:dark){${rules('dark')}}`;
}

/** The icon as SVG markup.  size is the width and height in pixels; themed false gives the standalone file. */
export function iconSVG({ size = 24, themed = true, label = '' } = {}) {
  const paint = themed ? (role) => `style="${ROLES[role].style}"` : (role) => `class="${role}"`;
  const aria = label ? `role="img" aria-label="${label}"` : 'aria-hidden="true"';
  const body = `${LINES.map((l) => lineMarkup(l, paint)).join('')}${MARKS.map((n) => markMarkup(n, paint)).join('')}`;
  const sheet = themed ? '' : `<style>${fileStyle()}</style>`;
  const xmlns = themed ? '' : ' xmlns="http://www.w3.org/2000/svg"';
  // Themed, the strokes' widths are set here; in the file, the style block sets them.
  const strokes = themed ? ` stroke-width="${OUTLINE}" stroke-linecap="round"` : '';
  const lines = themed ? body.replaceAll('<line ', `<line stroke-width="${LINE}" `) : body;
  return `<svg${xmlns} viewBox="0 0 ${BOX} ${BOX}" width="${size}" height="${size}" ${aria}${strokes}>${sheet}${lines}</svg>`;
}
