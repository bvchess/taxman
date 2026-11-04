// The data, loaded once a page: the chain's columns (games.json), the first games' moves (small.json), and one
// measurement that is in neither (START_5060).

const base = new URL('../data/', import.meta.url);
const cache = new Map();

function load(name) {
  if (!cache.has(name)) {
    cache.set(name, fetch(new URL(name, base)).then((r) => {
      if (!r.ok) throw new Error(`could not load ${name}: ${r.status}`);
      return r.json();
    }));
  }
  return cache.get(name);
}

/** games.json: { source, lastN, columns: { n, score, proof, method, nodes, seconds, moves, promotions,
 *  candidates, inTheBag, storeRule } }. */
export const loadGames = () => load('games.json');

/** small.json: [{ n, score, moves, promotions }] for the first games. */
export const loadSmall = () => load('small.json');

/** The start of game 5060's search, as a promotion sum: from the game of 5059 (seeded) and with no seed (own), measured
 *  by the frame solver (taxman-notebook, search/the-start-of-game-5060-with-and-without-the-game-before-2026-11). */
const START_5060 = { n: 5060, seeded: 570285, own: 569735 };

/** Where the last game's search starts, against its best, the game's score less its in_the_bag: the two starts and
 *  how far each lies below the best.  Throws if the last game is not the one measured. */
export function headStart(games) {
  if (games.lastN !== START_5060.n) {
    throw new Error(`the measured start is of game ${START_5060.n}, not of the last game (${games.lastN})`);
  }
  const c = games.columns;
  const i = games.lastN - 1;
  const best = c.score[i] - c.inTheBag[i];
  const { seeded, own } = START_5060;
  return { n: START_5060.n, best, seeded, own, seededGap: best - seeded, ownGap: best - own };
}
