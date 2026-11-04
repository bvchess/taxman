// The data, loaded once a page: the chain's columns (games.json) and the first games' moves (small.json).

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
 *  candidates, inTheBag } }. */
export const loadGames = () => load('games.json');

/** small.json: [{ n, score, moves, promotions }] for the first games. */
export const loadSmall = () => load('small.json');
