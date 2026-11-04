// What rests on what: the kinds of proof in the chain's records, grouped by what each rests on, and under them what
// makes the frame model's best score opt(n): the prover's proof up to 2175, statement U at n past it.  A flat
// figure in HTML, each count read from data/games.json.  The two foundations wear the standing pills they have, and
// the box of the one given U has the pill's double outline.
//
// The kinds and their meaning are those of solver/docs/REFERENCE.md, "Proof kinds"; the prover's reach is
// results/README.md's, "What supports the scores", item 6.

import { fmt, standingBadge } from '../js/vocab.js';
import { PROOFS } from './chain-chart.js';

/** The last game the prover proves on the whole board (results/README.md, "What supports the scores"). */
export const PROVER_TO = 2175;

const GROUPS = [
  { title: 'From the game before', kinds: ['cap', 'prime_formula'], rests: 'rests on game n − 1, which rests on its own proof' },
  { title: 'By a search of the frame model', kinds: ['exhausted', 'same_search', 'trivial'], rests: 'rests on the frame model of game n, and a repeat on the game it repeats' },
  { title: 'By a bound from an earlier run', kinds: ['known'], rests: 'rests on the frame model of the games the bound came from' },
];
const NAMES = { ...Object.fromEntries(PROOFS.map((p) => [p.key, p.name])), known: 'meets an upper bound an earlier run proved', trivial: 'nothing to choose' };

/** The counts the figure and its prose show: games by proof, and the range of the games proven by a bound. */
export function restsFacts(games) {
  const c = games.columns;
  const count = Object.fromEntries(PROOFS.map((p) => [p.key, c.proof.filter((x) => x === p.key).length]));
  const known = c.n.filter((_, i) => c.proof[i] === 'known');
  return { count, total: c.n.length, last: games.lastN, knownFrom: known[0], knownTo: known[known.length - 1], past: games.lastN - PROVER_TO };
}

export function mountRestsOn(host, { games }) {
  const f = restsFacts(games);
  if (GROUPS.flatMap((g) => g.kinds).length !== PROOFS.length || Object.values(f.count).reduce((a, b) => a + b, 0) !== f.total) {
    throw new Error('rests on: the kinds do not cover the chain');
  }
  const box = host.querySelector('.rests');
  const group = (g) => `<div class="rests-group">
      <p class="rests-title">${g.title}</p>
      <ul>${g.kinds.map((k) => `<li><span>${NAMES[k]}</span><b>${fmt(f.count[k])}</b></li>`).join('')}</ul>
      ${g.kinds.includes('known') ? `<p class="rests-range">games ${fmt(f.knownFrom)} to ${fmt(f.knownTo)}</p>` : ''}
      <p class="rests-on"><span class="rests-arrow" aria-hidden="true"></span>${g.rests}</p>
    </div>`;
  box.innerHTML = `
    <p class="rests-top">All ${fmt(f.total)} games: each replayed by the referee and by both verifiers</p>
    <div class="rests-groups">${GROUPS.map(group).join('')}</div>
    <div class="rests-model">
      <p class="rests-title">The frame model of game k gives opt(k)</p>
      <div class="rests-base">
        <div class="rests-proved"><p><b>k up to ${fmt(PROVER_TO)}</b>${standingBadge('proved')}</p><p>The prover proves opt(k) on the whole board, without the frame model.</p></div>
        <div class="rests-given"><p><b>k past ${fmt(PROVER_TO)}</b>${standingBadge('given-u')}</p><p>The frame model's best is opt(k) exactly when statement U holds at k.</p></div>
      </div>
    </div>`;
  return f;
}
