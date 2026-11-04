// Reads the committed chain of proven games, results/optimal.jsonl.gz, for export.mjs and test.mjs.
//
// The file is gzip with several members, one per batch of games; Node's gunzipSync decodes them all in sequence.

import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gunzipSync } from 'node:zlib';

/** The repository root, found from this script's own path (web/tools/ is two levels down). */
export const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..', '..');

/** The results file, relative to the repository root. */
export const resultsFile = 'results/optimal.jsonl.gz';

// The chain reached game 5060 when this was written, and it only grows.  A decode that stopped after the first member
// would end far short of it.
const MIN_LAST_N = 5060;

/** Every line of the results file, parsed: one record per game, for n = 1 up to the last game, in order.
 *  Throws if the file does not hold each n from 1 once, in order, or ends before MIN_LAST_N. */
export function readResults() {
  const text = gunzipSync(readFileSync(join(repoRoot, resultsFile))).toString('utf8');
  const records = text.split('\n').filter((line) => line !== '').map((line) => JSON.parse(line));
  records.forEach((record, i) => {
    if (record.n !== i + 1) throw new Error(`${resultsFile}: line ${i + 1} is game ${record.n}, not game ${i + 1}`);
  });
  const lastN = records[records.length - 1].n;
  if (lastN < MIN_LAST_N) throw new Error(`${resultsFile}: ends at game ${lastN}, before game ${MIN_LAST_N}`);
  return records;
}
