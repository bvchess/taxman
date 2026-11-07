// The tables of chapter 6, on two small sets.  For one set, a table has a row for each number and a column for each
// factor outside the set that a number lists, with a 1 where the number lists it.  A pick takes a 1 in every row,
// no two in a column: it is one payment scheme, and the outlined cells show it.  Choose rows to add up, and the
// bottom row says whether each column comes out even or odd; the set fails the parity test exactly when some group of
// rows is even in every column.  Every list, pick and sum is computed here from ladder.js, and the verdict is checked
// against a brute-force search of every group of rows.  Only the layout is written by hand.

import { listsOf, schemesOf, passesParityTest } from './ladder.js';
import { listWords, numberWord } from '../js/vocab.js';

const SETS = [[6, 10, 15], [9, 10, 15]];

/** Whether some nonempty group of rows adds up to even in every column, by trying every group. */
function someGroupIsEven(rows) {
  for (let mask = 1; mask < 1 << rows.length; mask++) {
    const chosen = rows.filter((_, i) => mask & (1 << i));
    if (chosen[0].every((_, c) => chosen.reduce((sum, row) => sum + row[c], 0) % 2 === 0)) return true;
  }
  return false;
}

function element(tag, props = {}, ...children) {
  const node = Object.assign(document.createElement(tag), props);
  node.append(...children);
  return node;
}

/** One set's block: its title, table, verdict and picks. */
function block(set) {
  const lists = listsOf(set);
  const columns = [...new Set([...lists.values()].flat())].sort((a, b) => a - b);
  const cells = set.map((x) => columns.map((f) => (lists.get(x).includes(f) ? 1 : 0)));
  const passes = passesParityTest(set);
  if (someGroupIsEven(cells) === passes) {
    throw new Error(`The parity test of ${set} disagrees with a search of every group of rows`);
  }
  const schemes = schemesOf(set);
  const title = listWords(set);
  const chosen = new Set(set);
  let pick = 0;

  const sums = columns.map(() => element('td', { className: 'pt-sum' }));
  const grid = element('tbody');
  const cellAt = set.map(() => []);
  set.forEach((x, r) => {
    const toggle = element('button', { type: 'button', className: 'btn pt-toggle' }, String(x));
    toggle.setAttribute('aria-pressed', 'true');
    toggle.setAttribute('aria-label', `Row of ${x}`);
    const row = element('tr', { className: 'is-chosen' }, element('th', { scope: 'row' }, toggle));
    columns.forEach((f, c) => {
      const cell = element('td', { className: `pt-cell${cells[r][c] ? ' is-one' : ''}` }, String(cells[r][c]));
      cellAt[r][c] = cell;
      row.append(cell);
    });
    toggle.addEventListener('click', () => {
      if (!chosen.delete(x)) chosen.add(x);
      toggle.setAttribute('aria-pressed', String(chosen.has(x)));
      row.classList.toggle('is-chosen', chosen.has(x));
      showSums();
    });
    grid.append(row);
  });
  function showSums() {
    columns.forEach((_, c) => {
      const total = set.reduce((sum, x, r) => sum + (chosen.has(x) ? cells[r][c] : 0), 0);
      sums[c].textContent = chosen.size === 0 ? '—' : `${total}, ${total % 2 ? 'odd' : 'even'}`;
    });
  }
  function showPick() {
    set.forEach((x, r) => columns.forEach((f, c) => {
      cellAt[r][c].classList.toggle('is-pick', schemes[pick].get(x) === f);
    }));
    buttons.forEach((b, i) => b.setAttribute('aria-pressed', String(i === pick)));
  }

  const table = element(
    'table',
    { className: 'pt-table' },
    element(
      'thead', {}, element('tr', {}, element('td'), ...columns.map((f) => element('th', { scope: 'col' }, `${f}`))),
    ),
    grid,
    element('tfoot', {}, element('tr', {}, element('th', { scope: 'row' }, 'sum'), ...sums)),
  );
  table.setAttribute('aria-label', `The table of ${title}`);
  table.querySelector('tfoot').setAttribute('aria-live', 'polite');

  const verdict = passes
    ? 'No group of rows adds up to even in every column: the set passes the parity test.'
    : 'Some rows add up to even in every column: the set fails the parity test.';
  const count = `${numberWord(schemes.length)} ${schemes.length === 1 ? 'pick' : 'picks'}`;
  const buttons = schemes.length < 2 ? [] : schemes.map((_, i) => {
    const b = element('button', { type: 'button', className: 'btn pt-pick' }, `pick ${i + 1}`);
    b.addEventListener('click', () => {
      pick = i;
      showPick();
    });
    return b;
  });

  showSums();
  showPick();
  return element(
    'div', { className: 'pt-block' },
    element('p', { className: 'pt-title' }, title),
    table,
    element('p', { className: 'pt-verdict' }, verdict),
    element('p', { className: 'pt-picks' }, count, ...buttons),
  );
}

/** Fill a host's `.parity-tables` with the two sets' blocks. */
export function mountParityTables(host) {
  const into = host.querySelector('.parity-tables');
  into.replaceChildren(...SETS.map(block));
  return { host: into };
}
