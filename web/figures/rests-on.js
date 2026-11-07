// What rests on what: the games of the chain by the rules of chapter 2, grouped by what each rests on, and under them what
// makes the frame model's best score opt(n): the prover's proof up to PROVER_TO (js/proof.js), statement U at n past
// it.  A flat figure in HTML, each count read from data/games.json.  The two foundations wear the standing pills they
// have, and the box of the one given statement U has the pill's double outline.
//
// The rules are those of js/proof.js (ruleOf); the prover's reach is
// results/README.md's, "What supports the scores", item 6.

import { fmt, standingBadge } from '../js/vocab.js';
import { PROVER_TO, RULE, RULES, foundationOf, ruleOf } from '../js/proof.js';

const GROUPS = [
  { title: 'From the game before', rules: ['ceiling', 'prime'], rests: 'rests on the score of game n − 1, and so on how that score was proven' },
  { title: 'By a search of the frame model', rules: ['repeat', 'search'], rests: 'rests on the frame model of game n, and a repeat on the game it repeats' },
];

/**
 * The counts the figure and its prose show: games by rule, as chapter 2 counts them.  A score rests on statement U
 * when its foundation (js/proof.js, foundationOf) is a game past PROVER_TO: onU counts those games, firstU is the
 * first.
 */
export function restsFacts(games) {
  const c = games.columns;
  const count = Object.fromEntries(RULES.map((r) => [r.key, 0]));
  let onU = 0;
  let firstU = null;
  c.n.forEach((n, i) => {
    count[ruleOf(c, i)] += 1;
    if (foundationOf(c, i) > PROVER_TO) {
      onU += 1;
      firstU ??= n;
    }
  });
  return { count, total: c.n.length, last: games.lastN, onU, firstU };
}

export function mountRestsOn(host, { games }) {
  const f = restsFacts(games);
  if (GROUPS.flatMap((g) => g.rules).length !== RULES.length || Object.values(f.count).reduce((a, b) => a + b, 0) !== f.total) {
    throw new Error('rests on: the rules do not cover the chain');
  }
  const box = host.querySelector('.rests');
  const group = (g) => `<div class="rests-group">
      <p class="rests-title">${g.title}</p>
      <ul>${g.rules.map((k) => `<li><span>${RULE[k].name}</span><b>${fmt(f.count[k])}</b></li>`).join('')}</ul>
      <p class="rests-on"><span class="rests-arrow" aria-hidden="true"></span>${g.rests}</p>
    </div>`;
  box.innerHTML = `
    <div class="rests-groups">${GROUPS.map(group).join('')}</div>
    <div class="rests-model">
      <p class="rests-title">The frame model of game k gives opt(k)</p>
      <div class="rests-base">
        <div class="rests-proved"><p><b>k up to game ${fmt(PROVER_TO)}</b>${standingBadge('proved')}</p><p>The prover proves opt(k) on the whole board, without the frame model.</p></div>
        <div class="rests-given"><p><b>k past game ${fmt(PROVER_TO)}</b>${standingBadge('given-u')}</p><p>The frame model's best is opt(k) exactly when statement U holds at k.</p></div>
      </div>
    </div>`;
  return f;
}
