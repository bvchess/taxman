// Along the way: game 425 has two optimal sets.  They play the same numbers above n/2, U*, and differ by an exchange
// of three promotions of equal sum, one into each of frames 2, 1 and 0.  The promotions are drawn flat, as the player's
// double circles, in the wiki's order of frames, the highest at the left.  The exchange is the theory's (docs/theory/
// uniqueness-and-longest-games.md, "Optimal sets on the whole board"), written out here and checked: the two sides
// have equal sums and pair off frame by frame; the score, U* and the count of promotions come from data/games.json.

import * as M from '../js/model.js';
import { fmt, numberSVG } from '../js/vocab.js';

const N = 425;
const ONE = [168, 132, 110];
const OTHER = [162, 150, 98];

const sum = (xs) => xs.reduce((a, b) => a + b, 0);

/** The numbers the page says of game 425, checked against the exchange. */
export function facts425(games) {
  const i = games.columns.n.indexOf(N);
  const model = M.frameModel(N);
  const frames = ONE.map((x) => M.promotedInto(model, x));
  const sameFrames = OTHER.every((x, j) => M.promotedInto(model, x) === frames[j]);
  if (sum(ONE) !== sum(OTHER) || !sameFrames || [...ONE, ...OTHER].some((x) => x > N / 2)) {
    throw new Error(`game ${N}: the exchange is not three numbers up to n/2 of equal sum, frame for frame`);
  }
  return {
    score: games.columns.score[i], inTheBag: games.columns.inTheBag[i], promotions: games.columns.promotions[i],
    shared: games.columns.promotions[i] - ONE.length, swap: sum(ONE), frames,
  };
}

/** Fill each span data-425="score" (or inTheBag, promotions, shared, swap) with that number. */
export function fill425(games, root = document) {
  const facts = facts425(games);
  for (const span of root.querySelectorAll('[data-425]')) span.textContent = fmt(facts[span.dataset['425']]);
  return facts;
}

export function mountTwoSets(host, { games }) {
  const facts = facts425(games);
  host.classList.add('exchange', 'two-sets');
  const head = document.createElement('div');
  head.className = 'exchange-row two-sets-head';
  head.setAttribute('aria-hidden', 'true');
  const headMarks = facts.frames.map((h) => `<span>${h}</span>`).join('');
  head.innerHTML = `<span>promoted into frame</span><div class="exchange-marks">${headMarks}</div><span>sum</span>`;
  host.appendChild(head);
  const row = (xs, label) => {
    const div = document.createElement('div');
    div.className = 'exchange-row';
    div.innerHTML = `<div class="exchange-label"><b>${label}</b><span>scores ${fmt(facts.score)}</span></div>`;
    const marks = document.createElement('div');
    marks.className = 'exchange-marks';
    for (const x of xs) {
      marks.appendChild(numberSVG(x, { rendering: 'flat', looks: ['move', 'player', 'candidate'], size: 42, pad: 3 }));
    }
    div.appendChild(marks);
    div.insertAdjacentHTML('beforeend', `<div class="exchange-sum">${fmt(sum(xs))}</div>`);
    return div;
  };
  host.append(row(ONE, 'One optimal set'), row(OTHER, 'The other'));
  host.setAttribute('role', 'img');
  host.setAttribute(
    'aria-label',
    `Game ${N}: one optimal set plays ${ONE.join(', ')}, the other ${OTHER.join(', ')}, one into each of frames ` +
      `${facts.frames.join(', ')}; each three sum to ${fmt(sum(ONE))}, and both sets score ${fmt(facts.score)}.`
  );
  return facts;
}
