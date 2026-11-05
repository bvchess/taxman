# The website

This folder is the project's explanatory website, *Taxman: Playing optimal games*.  Every page has three columns:
navigation on the left, the story in the center, and the technical layer on the right.  The pages are plain HTML, CSS
and ES modules, and need no build step.  [OUTLINE.md](OUTLINE.md) is the plan of the pages.

* `index.html` is the introduction, which explains the game.  The chapters of the story follow: `sets.html` (1),
  `chain.html` (2), `top-half.html` (3), `frames.html` (4), `search.html` (5), `trust.html` (6) and `results.html` (7).
  `along.html` lists the findings along the way, and `glossary.html`, `notation.html` and `about.html` are the reference
  pages.  `style.html` shows the site's look; no page links it.
* `css/site.css` holds the look of every page; a page with styles of its own has `css/<page>.css` beside it.
* `js/` holds the site's modules: `model.js` is the game, the frame model and playability, with names from the
  [glossary](../docs/GLOSSARY.md); `play.js` is the game the reader plays, apart from its board, which
  `figures/game.js` and `figures/graph-game.js` draw, over the two trays of `trays.js`; `proof.js` is the one place that knows how each game was proven
  (its rule and its group, from `data/games.json` alone); `site.js` lists the pages, and every navigation draws from it.
* `figures/` holds one file for each figure, `css/` and `fonts/` the look.
* `data/` holds `games.json`, a column for every field of every game in the chain, and `small.json`, the moves and
  promotions of games 1 to 100.
* `tools/` holds `export.mjs`, `results.mjs`, `test.mjs` and `icon.mjs`, which writes `icon.svg`, the favicon, from
  `js/icon.js`; run it after changing the icon.
* `img/` holds the pictures that the repository's README and the wiki link to.  Leave it as it is.

## Serve it

ES modules and `fetch` need http, so a page opened as `file://` does not work.  Serve this folder with any static
server:

```
python3 -m http.server 8780 --directory web
```

Then open `http://localhost:8780/`.

While we edit, we use `node web/tools/serve.mjs` instead.  It serves the same folder with caching off, so a reload
always gets the current scripts and styles together.  For a plain look, any static server works.

## Re-export the data and run the test

From the repository root, with Node and no packages:

```
node web/tools/export.mjs
node web/tools/test.mjs
```

Re-export when `results/optimal.jsonl.gz` changes, and commit the two JSON files.  The test replays every recorded
game and checks descending greedy against the recorded `in_the_bag`.  It prints one line, takes under half a minute,
and exits nonzero on a failure.

## Add a figure

A figure is one file in `figures/`: facts from `../js/model.js`, marks placed by hand, drawn by `js/scene.js`, which
keeps each mark one object across states.  A figure the reader plays is `raised`; one that explains is `flat`.  Both
read the looks of `js/vocab.js`: `shape` (0 a square, 1 a circle), `kept`, `dashed`, `ring`, `player`, `taxman`,
`halo`.  `style.html` shows all.

```js
// figures/pair.js
import { scene, timeline } from '../js/scene.js';

export function mountPair(host) {
  const s = scene(host.querySelector('svg'), { width: 200, height: 110, label: '12 and its maximal factor 6' });
  s.number(12, 12).number(6, 6).line('pay', 12, 6, 'pay');
  s.states([
    { 12: { x: 50, y: 40, shape: 1 }, 6: { x: 150, y: 70, shape: 0 } },
    { 12: { x: 50, y: 40, shape: 1, player: 1 }, 6: { x: 150, y: 70, shape: 0, taxman: 1 }, pay: { at: [0.3, 1] } },
  ]);
  timeline(s, host.querySelector('.fig-timeline'), {
    stops: [{ at: 0, name: 'pot' }, { at: 1, name: 'play' }],
    param: 'pair', fromParam: (v) => Number(v) - 1, toParam: (k) => String(k + 1),
  });
  return { goState: (k) => s.go(Number(k) - 1), stopName: (k) => ['pot', 'play'][k - 1], highlight: s.highlight };
}
```

```html
<figure class="fig" id="pair"><svg></svg><div class="fig-timeline"></div></figure>
<script type="module">
  import { initEssay, registerFigure } from './js/essay.js';
  import { mountPair } from './figures/pair.js';
  initEssay();
  registerFigure('pair', mountPair(document.getElementById('pair')));
</script>
```

`?pair=2` opens state 2; `<a class="go" data-fig="pair" data-state="2">` moves it there.  A played figure passes
`rendering: 'raised'` and moves by `s.morph(looks)`.
