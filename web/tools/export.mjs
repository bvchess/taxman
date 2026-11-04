// Exports the results file to the JSON the prototypes load, so they need no build step.
//
//   node web/tools/export.mjs
//
// Writes data/games.json (one column of numbers or strings per field, for every game in the chain) and data/small.json
// (the full records of games 1 to 100).  Run it again after the results file changes, and commit both.

import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { readResults, repoRoot, resultsFile } from './results.mjs';

const dataDir = join(repoRoot, 'web', 'data');
const SMALL_LAST_N = 100;

// A missing field is null.
const orNull = (value) => value ?? null;
const significant = (value, digits) => (value == null ? null : Number(value.toPrecision(digits)));

const records = readResults();
const lastN = records[records.length - 1].n;

// The columns, parallel arrays indexed by n - 1.
const columns = {
  n: records.map((r) => r.n),
  score: records.map((r) => orNull(r.score)),
  proof: records.map((r) => orNull(r.proof)),
  method: records.map((r) => orNull(r.method)),
  nodes: records.map((r) => orNull(r.nodes)),
  seconds: records.map((r) => significant(r.seconds?.total, 3)),
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

mkdirSync(dataDir, { recursive: true });
writeFileSync(join(dataDir, 'games.json'), `${games}\n`);
writeFileSync(join(dataDir, 'small.json'), `${smallText}\n`);
console.log(`exported games 1 to ${lastN} (results commit ${commit}) and games 1 to ${SMALL_LAST_N} to ${dataDir}`);
