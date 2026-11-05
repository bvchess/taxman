// Tests model.js against the definitions in the docs and against the results file.
//
//   node web/tools/test.mjs
//
// Prints a one-line summary, and exits nonzero if any check fails.  It takes about 20 seconds on a laptop, nearly all
// of it in descending greedy, whose cost grows with the cube of n.  The bounds below say what is covered.

import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

import {
  descendingGreedy, finish, frameModel, frames, isOver, isPlayable, legalMoves, maximalFactors, newGame, omega, peel,
  play, primeFactors, properDivisors, replay, score, sources,
} from '../js/model.js';
import { AFTER, INTRO, REFERENCE, STORY } from '../js/site.js';
import { fitsFacts, fitsWords, primeFacts, primeWords } from '../figures/fits.js';
import { slipFacts, slipOfRecord } from '../figures/slip.js';
import { GROUPS, RULES, oneSearchFacts, settled } from '../js/proof.js';
import { readResults, repoRoot } from './results.mjs';

// Descending greedy is checked against the results file's in_the_bag for every game up to GREEDY_ALL_UP_TO, then for
// the games in GREEDY_SPREAD.  Replay is much faster, so every game in the file is replayed.
const GREEDY_ALL_UP_TO = 500;
const GREEDY_SPREAD = [600, 750, 1000, 1500, 2000, 3000, 4000, 5000, 5040, 5059, 5060];
// Frames are checked against their lemma for every game up to FRAMES_UP_TO, and frameModel against in_the_bag up to
// FRAME_MODEL_UP_TO.
const FRAMES_UP_TO = 400;
const FRAME_MODEL_UP_TO = 150;
// Peel is checked against the game's own rules on every subset of 2..BRUTE_FORCE_N.
const BRUTE_FORCE_N = 12;

let checks = 0;
const failures = [];

function same(name, actual, expected) {
  checks++;
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    failures.push(`${name}: got ${JSON.stringify(actual)}, expected ${JSON.stringify(expected)}`);
  }
}

function throws(name, fn) {
  checks++;
  try {
    fn();
  } catch {
    return;
  }
  failures.push(`${name}: did not throw`);
}

const sum = (list) => list.reduce((total, x) => total + x, 0);
const started = performance.now();

// --- Number theory ----------------------------------------------------------------------------------------------

same('primeFactors(360)', primeFactors(360), [2, 3, 5]);
same('primeFactors(1)', primeFactors(1), []);
same('primeFactors(97)', primeFactors(97), [97]);
same('omega(12)', omega(12), 3);
same('omega(1)', omega(1), 0);
same('omega(97)', omega(97), 1);
same('omega(360)', omega(360), 6);
same('maximalFactors(12)', maximalFactors(12), [6, 4]);
same('maximalFactors(7)', maximalFactors(7), [1]);
same('maximalFactors(1)', maximalFactors(1), []);
same('properDivisors(12)', properDivisors(12), [1, 2, 3, 4, 6]);
same('properDivisors(36)', properDivisors(36), [1, 2, 3, 4, 6, 9, 12, 18]);
same('properDivisors(7)', properDivisors(7), [1]);
same('properDivisors(1)', properDivisors(1), []);
for (let m = 1; m <= 300; m++) {
  const expected = [];
  for (let d = 1; d < m; d++) if (m % d === 0) expected.push(d);
  same(`properDivisors(${m})`, properDivisors(m), expected);
}

// --- The game ---------------------------------------------------------------------------------------------------

{
  const start = newGame(6);
  same('legalMoves(newGame(6))', legalMoves(start), [2, 3, 4, 5, 6]);
  const afterFive = play(start, 5);
  same('play leaves its argument alone', [start.pot.size, start.player, start.log], [6, [], []]);
  same('tax of 5', afterFive.log, [{ move: 5, tax: [1] }]);
  same('legalMoves after 5', legalMoves(afterFive), [4, 6]);
  same('isOver at the start', isOver(start), false);
  throws('1 is never a legal move', () => play(start, 1));
  throws('a move out of the pot', () => play(afterFive, 5));
  throws('a move with no divisor left', () => play(afterFive, 3));
  throws('finish with moves left', () => finish(start));

  const done = replay(6, [5, 4, 6]);
  same('game 6, moves 5, 4, 6: score', score(done), 15);
  same('game 6, moves 5, 4, 6: log', done.log, [
    { move: 5, tax: [1] }, { move: 4, tax: [2] }, { move: 6, tax: [3] }, { rest: [] },
  ]);
  same('game 6, moves 5, 4, 6: taxman', done.taxman, [1, 2, 3]);
  same('game 6, moves 5, 4, 6: pot', done.pot.size, 0);
  same('isOver at the end', isOver(done), true);
  same('replay is play then finish', replay(6, [5, 4, 6]).log, finish(play(play(play(start, 5), 4), 6)).log);
  throws('replay with an illegal move', () => replay(6, [5, 4, 4]));
  throws('replay that stops early', () => replay(6, [5, 4]));
}

// --- The frame model --------------------------------------------------------------------------------------------

same('sources(21)', sources(21), [11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21]);
same('sources(20)', sources(20), [11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);

// The table in docs/theory/frame-model.md, section "Frames".
same('frames(21)', frames(21), [
  { layer: 1, sources: [11, 13, 17, 19], factors: [1] },
  { layer: 2, sources: [14, 15, 21], factors: [2, 3, 5, 7] },
  { layer: 3, sources: [12, 18, 20], factors: [4, 6, 9, 10] },
  { layer: 4, sources: [16], factors: [8] },
]);
same('frames(20), layer 2', frames(20).filter((frame) => frame.layer === 2), [
  { layer: 2, sources: [14], factors: [2, 7] },
  { layer: 2, sources: [15], factors: [3, 5] },
]);

// The frame lemma (F1): for n of 2 or more, every number of the board lies in exactly one frame, a frame's sources
// share a layer, and its factors lie one layer below.
for (let n = 2; n <= FRAMES_UP_TO; n++) {
  const times = new Array(n + 1).fill(0);
  const bad = [];
  for (const frame of frames(n)) {
    for (const x of [...frame.sources, ...frame.factors]) times[x]++;
    if (!frame.sources.every((s) => omega(s) === frame.layer)) bad.push(`sources of ${frame.sources[0]}`);
    if (!frame.factors.every((f) => omega(f) === frame.layer - 1 && f <= n / 2)) {
      bad.push(`factors of ${frame.sources[0]}`);
    }
  }
  for (let x = 1; x <= n; x++) if (times[x] !== 1) bad.push(`${x} is in ${times[x]} frames`);
  same(`frames(${n}) obey the frame lemma`, bad, []);
}

// Peel on game 21's U*: the steps pair every member once with a distinct maximal factor outside the set.
{
  const ustar = descendingGreedy(21);
  same('descendingGreedy(21)', ustar, [12, 14, 15, 16, 18, 19, 20, 21]);
  const run = peel(ustar);
  same('peel(U*(21)) is playable', [run.playable, run.stuck], [true, []]);
  same('peel(U*(21)) pairs every member once', run.steps.map((step) => step.member).sort((a, b) => a - b), ustar);
  same('peel(U*(21)) uses distinct factors', new Set(run.steps.map((step) => step.factor)).size, ustar.length);
  same('peel(U*(21)) pairs with maximal factors outside the set',
    run.steps.every((step) => maximalFactors(step.member).includes(step.factor) && !ustar.includes(step.factor)), true);
  same('frameModel(21)', frameModel(21).map((f) => [f.layer, f.kept, f.rejected, f.freedoms, f.closed]), [
    [1, [19], [11, 13, 17], 0, true],
    [2, [14, 15, 21], [], 1, false],
    [3, [12, 18, 20], [], 1, false],
    [4, [16], [], 0, true],
  ]);
}

// Peel that chooses its own steps reaches the same answer, whichever steps it takes (the peel theorem): on game 21's
// optimal set, taking the last step offered, a back move whenever there is one, still pairs every member once.
{
  const set = [9, 12, 14, 15, 16, 18, 19, 20, 21];
  let offered = true;
  const run = peel(set, {
    choose(options) {
      offered &&= options.every((o) => (o.kind === 'front' || o.kind === 'back') && set.includes(o.member));
      return options[options.length - 1];
    },
  });
  same('peel with choose, on game 21\'s optimal set, is playable', [run.playable, run.stuck], [true, []]);
  same('peel with choose offers only steps of its members', offered, true);
  same('peel with choose pairs every member once', run.steps.map((step) => step.member).sort((a, b) => a - b), set);
  same('peel with choose uses distinct factors', new Set(run.steps.map((step) => step.factor)).size, set.length);
  same('peel with choose, on the square, is stuck', peel([10, 14, 15, 21], { choose: (options) => options[0] }).stuck, [10, 14, 15, 21]);
  throws('peel with a choose that invents a step', () => peel(set, { choose: () => ({ kind: 'front', member: 9, factor: 1 }) }));
}

// The square of docs/theory/playability.md, "Payment schemes": 10, 14, 15 and 21 form a cycle on the primes 2, 3, 5, 7.
same('isPlayable([10, 14, 15, 21])', isPlayable([10, 14, 15, 21]), false);
same('isPlayable([10, 14, 15])', isPlayable([10, 14, 15]), true);
same('peel([10, 14, 15, 21]) is stuck on all four', peel([10, 14, 15, 21]).stuck, [10, 14, 15, 21]);
same('1 is never playable', isPlayable([1]), false);
same('the empty set is playable', isPlayable([]), true);

// Peel against the rules of the game itself.  A set is playable when some order of its members is a legal run of
// moves from the full pot.  Extra moves in between could only remove more numbers, so they never help.  The pot after
// a set of moves does not depend on their order (a number is gone exactly when it divides a move played), so the
// search can remember the sets it has seen.
function playableByTheRules(n, members) {
  const failed = new Set();
  function search(state, left) {
    if (left.length === 0) return true;
    const key = left.join(',');
    if (failed.has(key)) return false;
    const open = new Set(legalMoves(state));
    for (const m of left) {
      if (open.has(m) && search(play(state, m), left.filter((x) => x !== m))) return true;
    }
    failed.add(key);
    return false;
  }
  return search(newGame(n), members);
}
{
  const numbers = [];
  for (let m = 2; m <= BRUTE_FORCE_N; m++) numbers.push(m);
  const disagree = [];
  for (let mask = 0; mask < 2 ** numbers.length; mask++) {
    const members = numbers.filter((_, i) => mask & (1 << i));
    if (isPlayable(members) !== playableByTheRules(BRUTE_FORCE_N, members)) disagree.push(members);
    checks++;
  }
  same(`peel agrees with the rules on every subset of 2..${BRUTE_FORCE_N}`, disagree, []);
}

// --- The results file -------------------------------------------------------------------------------------------

const records = readResults();
const lastN = records[records.length - 1].n;

// Every recorded game replays legally, is over at its end, and scores the recorded score.
for (const record of records) {
  checks++;
  try {
    const got = score(replay(record.n, record.moves));
    if (got !== record.score) failures.push(`replay of game ${record.n} scores ${got}, recorded ${record.score}`);
  } catch (error) {
    failures.push(`replay of game ${record.n}: ${error.message}`);
  }
}

// Descending greedy keeps U*, and the sum of U* is the recorded in_the_bag (null where the game was settled without
// setup, as primes are).
const greedyGames = [];
for (let n = 1; n <= GREEDY_ALL_UP_TO; n++) greedyGames.push(n);
greedyGames.push(...GREEDY_SPREAD.filter((n) => n <= lastN));
let compared = 0;
for (const n of greedyGames) {
  const inTheBag = records[n - 1].in_the_bag;
  if (inTheBag == null) continue;
  compared++;
  same(`sum of descendingGreedy(${n}) is in_the_bag`, sum(descendingGreedy(n)), inTheBag);
}
for (let n = 1; n <= FRAME_MODEL_UP_TO; n++) {
  const inTheBag = records[n - 1].in_the_bag;
  if (inTheBag == null) continue;
  const kept = frameModel(n).flatMap((frame) => frame.kept);
  same(`the kept sources of frameModel(${n}) sum to in_the_bag`, sum(kept), inTheBag);
}

// --- How each game was proven (js/proof.js) ---------------------------------------------------------------------

// The committed data/games.json is what the site's figures sort, so it must be this results file's export: the seconds
// over every run of a search, and the store rule of a line of proof known.
const games = JSON.parse(readFileSync(join(repoRoot, 'web', 'data', 'games.json'), 'utf8'));
const cols = games.columns;
same('games.json holds every game of the results file', cols.n.length, records.length);
const stale = [];
records.forEach((r, i) => {
  const t = r.seconds;
  const seconds = Number((t.total - t.search + t.search_all_runs).toPrecision(3));
  const storeRule = r.proof === 'known' ? r.known.upper.rule : null;
  if (t.search_all_runs < t.search) stale.push(`game ${r.n}: search_all_runs is less than search`);
  if (cols.seconds[i] !== seconds) stale.push(`game ${r.n}: seconds ${cols.seconds[i]}, expected ${seconds}`);
  if (cols.storeRule[i] !== storeRule) stale.push(`game ${r.n}: storeRule ${cols.storeRule[i]}, expected ${storeRule}`);
  if (cols.proof[i] !== r.proof || cols.score[i] !== r.score) stale.push(`game ${r.n}: proof or score differs`);
});
same('games.json is the export of the results file (run node web/tools/export.mjs)', stale.slice(0, 5), []);

// Every game has exactly one rule (ruleOf throws on one it does not cover), and the groups add up to all of them.
const sorted = settled(games);
same('every game has a rule', sorted.rules.every((rule) => RULES.some((r) => r.key === rule)), true);
same('the rules add up to every game', sum(Object.values(sorted.counts)), records.length);
same('the groups add up to every game', sum(Object.values(sorted.groupCounts)), records.length);
const badGroup = [];
const badSearch = [];
sorted.rules.forEach((rule, i) => {
  const { proof, storeRule } = { proof: cols.proof[i], storeRule: cols.storeRule[i] };
  const before = proof === 'cap' || proof === 'prime_formula' || (proof === 'known' && ['ceiling', 'prime'].includes(storeRule));
  if (before && sorted.groups[i] !== 'before') badGroup.push(cols.n[i]);
  const ok = proof === 'exhausted' || proof === 'trivial' || (proof === 'known' && storeRule === 'exhausted');
  if (sorted.groups[i] === 'search' && !ok) badSearch.push(cols.n[i]);
  if (sorted.groups[i] === 'repeat' && !(proof === 'same_search' || (proof === 'known' && storeRule === 'repeat'))) {
    badSearch.push(cols.n[i]);
  }
});
same('every cap, prime formula and store ceiling or prime record is derived from n − 1', badGroup.slice(0, 5), []);
same('every game of the search group is exhausted, trivial or a store search, and of the repeat group a repeat',
  badSearch.slice(0, 5), []);
const groupLine = GROUPS.map((g) => `${g.name} ${sorted.groupCounts[g.key].toLocaleString('en-US')}`).join(', ');
let runs = null;
try {
  runs = oneSearchFacts(games);
} catch (e) {
  failures.push(`oneSearchFacts: ${e.message}`);
}
checks++;
const ruleLine = RULES.map((r) => `${r.key} ${sorted.counts[r.key].toLocaleString('en-US')}`).join(', ');

// --- The figures of chapter 2 that draw games as rows (figures/slip.js, figures/fits.js) ---------------------------

const smallGames = JSON.parse(readFileSync(join(repoRoot, 'web', 'data', 'small.json'), 'utf8'));
const slip12 = slipFacts(12, [11, 9, 6, 10, 8]);
same('slipFacts(12): the game', slip12.after.moves, [11, 9, 6, 10, 12]);
same('slipFacts(12): 12 takes 4, 8 is left out', [slip12.after.turns[4].tax, slip12.dropped, slip12.cut, slip12.last], [[4], 8, 8, 4]);
same('slipFacts(12): the score', [slip12.before.score, slip12.after.score, slip12.gain], [44, 48, 4]);
same('slipOfRecord(12): the same game from the record', slipOfRecord(smallGames, 12).after.moves, slip12.after.moves);
const slip10 = slipFacts(10, [7, 9, 6, 8]);
same('slipFacts(10): 10 goes last, nothing is left out', [slip10.after.moves, slip10.dropped, slip10.before], [[7, 9, 6, 8, 10], null, null]);
for (const [n, fits, below, words] of [[10, true, [7, 9, 6, 8, 10], '30 + 10 = 40, the ceiling.'], [12, false, [11, 9, 10, 8, 12], '44 + 12 − 6 = 50.']]) {
  const f = fitsFacts(smallGames, n);
  same(`fitsFacts(${n}): fits, the game below, what pays for n`, [f.fits, f.below.moves, f.uses], [fits, below, fits ? [5] : [4, 6]]);
  same(`fitsWords(${n}) ends with the scores`, fitsWords(f).endsWith(words), true);
}
same('fitsFacts(12): the columns of both rows', fitsFacts(smallGames, 12).slots, [11, 9, 6, 10, 8, 12]);
same('fitsWords(10) says what the game above leaves out', fitsWords(fitsFacts(smallGames, 10)),
  "<strong>10 fits.</strong> Game 9's optimal game leaves 5 out, and 10 follows it, paid for by 5: 30 + 10 = 40, the ceiling.");
same('slipFacts(12): its maximal factors', slip12.factors, [4, 6]);
const prime13 = primeFacts(smallGames, 13);
same('primeFacts(13): q, the rows, what n takes', [prime13.q, prime13.above.moves, prime13.below.moves, prime13.tax],
  [11, [11, 9, 10, 8, 12], [13, 9, 10, 8, 12], [1]]);
same('primeFacts(13): the scores', [prime13.above.score, prime13.below.score], [50, 52]);
same('primeFacts(13): the columns of each row', prime13.slots, [[11, 9, 10, 8, 12], [13, 9, 10, 8, 12]]);
same('primeWords(13)', primeWords(prime13),
  '<strong>13 is a prime.</strong>  Its only maximal factor is 1, so it can only be the first move.  It takes the '
  + "place of 11, the first move of game 12's optimal game, and every other move stays the same: 50 + 13 − 11 = 52.");
let notPrime = null;
try {
  primeFacts(smallGames, 12);
} catch (e) {
  notPrime = e.message;
}
same('primeFacts(12) throws: 12 is not a prime', notPrime, 'prime panel of 12: n is not a prime');

// --- The links at the foot of each page (js/site.js, js/essay.js) ---------------------------------------------------

// Every page of the map has exactly one empty nav.chapter-nav, which essay.js fills from the map; no page has a
// hand-written one, and none has a provenance line.  A page outside the map (style.html) has none.
const webDir = join(repoRoot, 'web');
const mapped = new Set([INTRO, ...STORY, ...AFTER, ...REFERENCE].map((p) => p.href).filter(Boolean));
for (const file of readdirSync(webDir).filter((f) => f.endsWith('.html'))) {
  const html = readFileSync(join(webDir, file), 'utf8');
  const navs = [...html.matchAll(/<nav\b[^>]*\bclass="chapter-nav"[^>]*>(.*?)<\/nav>/gs)];
  same(`${file}: nav.chapter-nav, empty`, mapped.has(file) ? navs.map((m) => m[1].trim()) : [], mapped.has(file) ? [''] : []);
  same(`${file}: no .provenance element`, mapped.has(file) ? /provenance/.test(html) : false, false);
  same(`${file}: no nav.chapter-nav outside the map`, mapped.has(file) ? 0 : navs.length, 0);
}

// --- Summary ----------------------------------------------------------------------------------------------------

const seconds = ((performance.now() - started) / 1000).toFixed(1);
for (const failure of failures.slice(0, 20)) console.error(`FAIL ${failure}`);
if (failures.length > 20) console.error(`... and ${failures.length - 20} more`);
if (failures.length > 0) {
  console.log(`FAILED: ${failures.length} of ${checks} checks failed, in ${seconds} s`);
  process.exit(1);
}
console.log(
  `ok: ${checks} checks passed in ${seconds} s (replayed games 1 to ${lastN}; ` +
    `descending greedy against in_the_bag on ${compared} games up to ${Math.max(...greedyGames)})`,
);
console.log(`proofs of ${records.length} games, by group: ${groupLine}; by rule: ${ruleLine}`);
if (runs) {
  const run = (r) => `games ${r.first} to ${r.last} (${r.count} games, ${r.nodes.toLocaleString('en-US')} nodes)`;
  console.log(`one search: the example ${run(runs.example)}; the longest ${run(runs.longest)}`);
}
