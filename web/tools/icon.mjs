// Writes web/icon.svg, the site's favicon, from js/icon.js: fixed light colors, and inside the file a dark-scheme
// block that swaps the ink and the paper.  Run it after changing js/icon.js.  With a size argument it prints the file
// at that size to stdout instead of writing it, for rendering the picture.

import { writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { iconSVG } from '../js/icon.js';

const out = join(dirname(fileURLToPath(import.meta.url)), '..', 'icon.svg');
const size = Number(process.argv[2]);
if (size) {
  process.stdout.write(iconSVG({ size, themed: false }));
} else {
  writeFileSync(out, `${iconSVG({ size: 32, themed: false, label: 'Taxman' })}\n`);
}
