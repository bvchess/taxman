// Exports the results file to the JSON the prototypes load, so they need no build step.
//
//   node web/tools/export.mjs
//
// Writes data/games.json (one column of numbers or strings per field, for every game in the chain), data/small.json
// (the full records of games 1 to 100) and data/bounds.json (the two upper bounds at the root of every game from 2).
// Run it again after the results file changes, and commit all three.
//
// The bounds come from the prover's two data files: its proofs of games 2 to 2531, and the root bounds of the later
// games (solver/noframe/docs/reproducing.md).  Each file's score must be the results file's.
//
// The seconds column is seconds.active_all_runs: the time the game had at least one worker, over every invocation that
// searched it (solver/frame/src/solver/window.rs, "The clock").

import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { readResults, repoRoot, resultsFile } from './results.mjs';

const dataDir = join(repoRoot, 'web', 'data');
const SMALL_LAST_N = 100;

// A missing field is null.
const orNull = (value) => value ?? null;
const significant = (value, digits) => (value == null ? null : Number(value.toPrecision(digits)));

// The game's seconds.active_all_runs (see the top of the file).
function activeSeconds(record) {
  const seconds = record.seconds?.active_all_runs;
  if (seconds == null) throw new Error(`game ${record.n}: its line lacks seconds.active_all_runs`);
  return seconds;
}

// The prover's lines that give each game's score and the two bounds at its root, by n.
const BOUNDS_FILES = ['exact_parity_2_2531.jsonl', 'rootbounds_2532_5060.jsonl'].map((name) =>
  join('solver', 'noframe', 'data', name));

function readBounds() {
  const byN = new Map();
  for (const file of BOUNDS_FILES) {
    const lines = readFileSync(join(repoRoot, file), 'utf8').split('\n').filter((line) => line.trim() !== '');
    for (const line of lines) {
      const r = JSON.parse(line);
      if (byN.has(r.n)) throw new Error(`${file}: game ${r.n} is in two lines`);
      byN.set(r.n, r);
    }
  }
  return byN;
}

const records = readResults();
const lastN = records[records.length - 1].n;

// The columns, parallel arrays indexed by n - 1.
const columns = {
  n: records.map((r) => r.n),
  score: records.map((r) => orNull(r.score)),
  proof: records.map((r) => orNull(r.proof)),
  method: records.map((r) => orNull(r.method)),
  nodes: records.map((r) => orNull(r.nodes)),
  seconds: records.map((r) => significant(activeSeconds(r), 3)),
  moves: records.map((r) => (r.moves == null ? null : r.moves.length)),
  promotions: records.map((r) => (r.promotions == null ? null : r.promotions.length)),
  candidates: records.map((r) => orNull(r.setup?.candidates)),
  inTheBag: records.map((r) => orNull(r.in_the_bag)),
};

const commit = execFileSync('git', ['log', '-1', '--format=%h', '--', resultsFile], { cwd: repoRoot })
  .toString()
  .trim();
// The date in Pacific time, as year-month-day.
const exported = new Date().toLocaleDateString('en-CA', { timeZone: 'America/Los_Angeles' });
const source = { file: resultsFile, commit, exported };

// One line a column and one a game keep the files readable and their diffs small.
const games = [
  `{"source":${JSON.stringify(source)},"lastN":${lastN},"columns":{`,
  Object.entries(columns)
    .map(([name, values]) => `${JSON.stringify(name)}:${JSON.stringify(values)}`)
    .join(',\n'),
  '}}',
].join('\n');

const small = records.slice(0, SMALL_LAST_N).map((r) => ({
  n: r.n,
  score: r.score,
  moves: r.moves,
  promotions: r.promotions,
}));
const smallText = `[\n${small.map((game) => JSON.stringify(game)).join(',\n')}\n]`;

// The bounds as parallel columns, indexed by n - 2.
const boundsByN = readBounds();
const bounds = { n: [], fm: [], parity: [] };
for (const record of records.slice(1)) {
  const b = boundsByN.get(record.n);
  if (b === undefined) throw new Error(`game ${record.n}: no line in ${BOUNDS_FILES.join(' or ')}`);
  if (b.score !== record.score) throw new Error(`game ${record.n}: the prover's file has score ${b.score}, ${resultsFile} ${record.score}`);
  if (!Number.isInteger(b.fm_bound) || !Number.isInteger(b.parity_bound)) throw new Error(`game ${record.n}: a bound is missing`);
  bounds.n.push(record.n);
  bounds.fm.push(b.fm_bound);
  bounds.parity.push(b.parity_bound);
}
if (boundsByN.size !== records.length - 1) {
  throw new Error(`the prover's files hold ${boundsByN.size} games, ${records.length - 1} expected (games 2 to ${lastN})`);
}
const boundsText = [
  `{"files":${JSON.stringify(BOUNDS_FILES)},"lastN":${lastN},"columns":{`,
  Object.entries(bounds).map(([name, values]) => `${JSON.stringify(name)}:${JSON.stringify(values)}`).join(',\n'),
  '}}',
].join('\n');

mkdirSync(dataDir, { recursive: true });
writeFileSync(join(dataDir, 'games.json'), `${games}\n`);
writeFileSync(join(dataDir, 'small.json'), `${smallText}\n`);
writeFileSync(join(dataDir, 'bounds.json'), `${boundsText}\n`);
console.log(`exported games 1 to ${lastN} (results commit ${commit}) and games 1 to ${SMALL_LAST_N} to ${dataDir}, and the bounds of games 2 to ${lastN}`);
