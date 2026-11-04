// Along the way: in game 833 a longer game is not a better one.  The moves are drawn flat, as the player's.  The optimal game's length and score come from
// data/games.json; the exchange of the length proposition (docs/theory/uniqueness-and-longest-games.md,
// "The length proposition") is its statement, written out here and checked: the two sides differ by one move and by
// exactly what the two scores differ by.

import { fmt, numberSVG } from '../js/vocab.js';

const N = 833;
// Proposition Q4: a heaviest longest game plays these five in place of the optimal game's four, and scores 218,197.
const OPTIMAL_PLAYS = [250, 252, 294, 400];
const LONGEST_PLAYS = [116, 180, 200, 320, 351];
const LONGEST_SCORE = 218197;

export function mountLonger833(host, { games }) {
  const i = games.columns.n.indexOf(N);
  const moves = games.columns.moves[i];
  const score = games.columns.score[i];
  const sum = (xs) => xs.reduce((a, b) => a + b, 0);
  if (sum(OPTIMAL_PLAYS) - sum(LONGEST_PLAYS) !== score - LONGEST_SCORE || LONGEST_PLAYS.length !== OPTIMAL_PLAYS.length + 1) {
    throw new Error('game 833: the exchange does not match the recorded score');
  }
  const row = (xs, label, total, count) => {
    const div = document.createElement('div');
    div.className = 'exchange-row';
    const coins = document.createElement('div');
    coins.className = 'exchange-marks';
    for (const x of xs) coins.appendChild(numberSVG(x, { rendering: 'flat', looks: ['move', 'player'], size: 42, pad: 3 }));
    div.innerHTML = `<div class="exchange-label"><b>${label}</b><span>${count} moves, ${fmt(total)}</span></div>`;
    div.appendChild(coins);
    div.insertAdjacentHTML('beforeend', `<div class="exchange-sum">${fmt(sum(xs))}</div>`);
    return div;
  };
  host.classList.add('exchange');
  host.append(
    row(OPTIMAL_PLAYS, 'Every optimal game', score, moves),
    row(LONGEST_PLAYS, 'A heaviest longest game', LONGEST_SCORE, moves + 1),
  );
  host.setAttribute('role', 'img');
  host.setAttribute('aria-label', `Game ${N}: an optimal game plays ${OPTIMAL_PLAYS.join(', ')}, which sum to ${fmt(sum(OPTIMAL_PLAYS))}, in ${moves} moves; a heaviest game of ${moves + 1} moves plays ${LONGEST_PLAYS.join(', ')} instead, which sum to ${fmt(sum(LONGEST_PLAYS))}.`);
  return { moves, score, longest: LONGEST_SCORE };
}
