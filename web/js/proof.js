// How each game of the chain was proven: the one place that knows.  Every figure that sorts the games by their proof
// takes the sorting from here, from data/games.json alone.
//
// The record of a game has a proof kind (PROOFS), and a game proven by the bounds store (kind known) also has the rule
// of the upper bound it cites, the column storeRule.  The story's classification is coarser and goes by the rule that
// the chapter "Walking up the chain" meets, the first that applies (ruleOf):
//
//   prime    n is prime (the prime formula, from game n − 1)
//   ceiling  n > 1 and opt(n) = n + opt(n − 1), the game's score is its ceiling (the ceiling lemma, from game n − 1)
//   repeat   the record's proof is same_search, or known with the store rule repeat
//   search   every other game: proof exhausted (its own search), known with the store rule exhausted (a search in a
//            run of the bounds store), and trivial (a game with nothing to promote, whose tree is a single node)
//
// and the front page shows three groups of them (GROUPS): search, repeats an earlier search, and derived from n − 1
// (the rules prime and ceiling).

import { primeFactors } from './model.js';
import { fmt } from './vocab.js';

/** The proof kinds of a results line, in plain words (docs/GLOSSARY.md, "proof"). */
export const PROOFS = [
  { key: 'exhausted', name: 'searched the whole tree', short: 'a search of the whole tree' },
  { key: 'same_search', name: 'repeats an earlier search', short: 'a repeat of an earlier game’s search' },
  { key: 'cap', name: 'reaches the ceiling', short: 'the ceiling' },
  { key: 'prime_formula', name: 'prime formula', short: 'the prime formula' },
  { key: 'known', name: 'proven in the bounds store', short: 'a game the bounds store proved' },
  { key: 'trivial', name: 'trivial: nothing to promote', short: 'trivial' },
];

/** The rules, in the order the chapter meets them.  proof is the record's proof kind that is the rule's own. */
export const RULES = [
  { key: 'ceiling', name: 'at the ceiling', proof: 'cap' },
  { key: 'prime', name: 'the prime formula', proof: 'prime_formula' },
  { key: 'repeat', name: 'a repeat', proof: 'same_search' },
  { key: 'search', name: 'a search', proof: 'exhausted' },
];
export const RULE = Object.fromEntries(RULES.map((r) => [r.key, r]));
/** The rules that settle a game with no search. */
export const NO_SEARCH = ['ceiling', 'prime', 'repeat'];

/** The front page's three groups of rules, in the order it shows them. */
export const GROUPS = [
  { key: 'search', name: 'search', rules: ['search'] },
  { key: 'repeat', name: 'repeats an earlier search', rules: ['repeat'] },
  { key: 'before', name: 'derived from game n − 1', rules: ['prime', 'ceiling'] },
];
export const GROUP = Object.fromEntries(GROUPS.map((g) => [g.key, g]));
const GROUP_OF_RULE = Object.fromEntries(GROUPS.flatMap((g) => g.rules.map((r) => [r, g.key])));

/** The group of a rule. */
export const groupOfRule = (rule) => GROUP_OF_RULE[rule];

export const isPrime = (n) => n >= 2 && primeFactors(n)[0] === n;

/** The rule of the game in row i of games.json's columns c.  Throws on a record the rules do not cover. */
export function ruleOf(c, i) {
  const n = c.n[i];
  const proof = c.proof[i];
  const stored = proof === 'known' ? c.storeRule[i] : null;
  if (isPrime(n)) return 'prime';
  if (n > 1 && c.score[i] === c.score[i - 1] + n) return 'ceiling';
  if (proof === 'same_search' || stored === 'repeat') return 'repeat';
  if (proof === 'exhausted' || proof === 'trivial' || stored === 'exhausted') return 'search';
  throw new Error(`game ${n}: proof ${proof}${stored ? ` by the store rule ${stored}` : ''} fits no rule`);
}

/** The game whose search a repeat n repeats: the game before it, or the one before that when n − 1 is prime, since a
 *  prime game runs no search (results/README.md, the paragraph on repeats, which every repeat of the chain keeps). */
export const repeatOf = (n) => (isPrime(n - 1) ? n - 2 : n - 1);

// For each game, the game whose search its record rests on: itself, or, for a record same_search, the root of the game
// it repeats (repeatOf).  Cached for the columns of one games.json.
const ROOTS = new WeakMap();
function rootsOf(c) {
  if (!ROOTS.has(c)) {
    const roots = [];
    c.n.forEach((n, i) => roots.push(c.proof[i] === 'same_search' ? roots[repeatOf(n) - 1] : n));
    ROOTS.set(c, roots);
  }
  return ROOTS.get(c);
}

/** The game whose search the record of row i rests on: game 2512 for each of games 2512 to 2519. */
export const searchOf = (c, i) => rootsOf(c)[i];

const STORE_WORDS = {
  exhausted: 'a search in the bounds store',
  repeat: 'a repeat in the bounds store',
  ceiling: 'the ceiling, by the bounds store',
  prime: 'the prime formula, by the bounds store',
};

/** How the chain's record proves the game in row i, in words: "a repeat of game 21's search". */
export function recordWords(c, i) {
  const n = c.n[i];
  switch (c.proof[i]) {
    case 'prime_formula': return 'the prime formula';
    case 'cap': return 'the ceiling';
    case 'same_search': return `a repeat of game ${fmt(searchOf(c, i))}’s search`;
    case 'exhausted': return `a search of ${fmt(c.nodes[i])} node${c.nodes[i] === 1 ? '' : 's'}`;
    case 'trivial': return 'nothing to choose';
    case 'known': return STORE_WORDS[c.storeRule[i]] ?? 'the bounds store';
    default: return c.proof[i];
  }
}

/** A count of nodes in words: 3,100,000 as "3.1 million", 940 as "940". */
export function nodesWords(k) {
  if (k < 1e6) return fmt(k);
  const [unit, name] = k < 1e9 ? [1e6, 'million'] : k < 1e12 ? [1e9, 'billion'] : [1e12, 'trillion'];
  return `${Number((k / unit).toPrecision(2))} ${name}`;
}

/** How the game in row i was proven by its rule, as one phrase that reads after "proven by" in a tooltip. */
export function ruleWords(c, i, rule = ruleOf(c, i)) {
  const stored = c.proof[i] === 'known';
  switch (rule) {
    case 'prime': return 'the prime formula';
    case 'ceiling': return 'the ceiling';
    case 'repeat': return stored ? 'a repeat, in the bounds store' : recordWords(c, i);
    default:
      if (stored) return 'a search, in the bounds store';
      if (c.proof[i] === 'trivial') return 'a search of one node';
      return `a search of ${nodesWords(c.nodes[i])} node${c.nodes[i] === 1 ? '' : 's'}`;
  }
}

/** The games that rest on one search of the chain: game n, which the chain searched, and every later game whose
 *  record repeats a search (same_search) and whose repeats, followed back through repeatOf, end at n. */
export function oneSearch(c, n) {
  if (c.method[n - 1] !== 'search' || c.proof[n - 1] !== 'exhausted') {
    throw new Error(`the chain did not search game ${n}`);
  }
  const roots = rootsOf(c);
  const games = c.n.filter((m, i) => m >= n && roots[i] === n);
  return { first: n, last: Math.max(...games), count: games.length, nodes: c.nodes[n - 1], games };
}

// The longest run of games on one search, which the words of chain.html name: games 2512 to 2519.
const LONGEST_FIRST = 2512;
const LONGEST_COUNT = 8;

/** The longest run of the chain, which must be the one the words name, games 2512 to 2519. */
export function oneSearchFacts(games) {
  const c = games.columns;
  let longest = null;
  c.n.forEach((n, i) => {
    if (c.method[i] !== 'search' || c.proof[i] !== 'exhausted') return;
    const run = oneSearch(c, n);
    if (!longest || run.count > longest.count) longest = run;
  });
  const last = LONGEST_FIRST + LONGEST_COUNT - 1;
  if (longest.first !== LONGEST_FIRST || longest.count !== LONGEST_COUNT || longest.last !== last) {
    const named = `${LONGEST_FIRST} to ${last}`;
    throw new Error(`the longest run is games ${longest.first} to ${longest.last}, not ${named}`);
  }
  return { longest };
}

/** Fill each span data-run="key" with that fact. */
export function fillOneSearch(games, root = document) {
  const { longest } = oneSearchFacts(games);
  const facts = {
    longestFirst: fmt(longest.first), longestLast: fmt(longest.last), longestCount: fmt(longest.count),
    longestNodes: nodesWords(longest.nodes), longestOthers: fmt(longest.count - 1),
  };
  for (const s of root.querySelectorAll('[data-run]')) s.textContent = facts[s.dataset.run];
}

/** Every game's rule and group, and the count of each. */
export function settled(games) {
  const c = games.columns;
  const rules = c.n.map((_, i) => ruleOf(c, i));
  const groups = rules.map(groupOfRule);
  const counts = Object.fromEntries(RULES.map((r) => [r.key, 0]));
  for (const r of rules) counts[r]++;
  const groupCounts = Object.fromEntries(GROUPS.map((g) => [g.key, g.rules.reduce((s, r) => s + counts[r], 0)]));
  const noSearch = NO_SEARCH.reduce((s, k) => s + counts[k], 0);
  // The games of the bounds store under each group, and two kinds of game the record and the rules sort differently.
  const count = (test) => rules.filter((_, i) => test(i)).length;
  const known = (i) => c.proof[i] === 'known';
  const store = Object.fromEntries(GROUPS.map((g) => [g.key, count((i) => groups[i] === g.key && known(i))]));
  const recordRepeatAtCeiling = count((i) => rules[i] === 'ceiling' && c.proof[i] === 'same_search');
  const searchNothing = count((i) => rules[i] === 'search' && c.proof[i] === 'trivial');
  return { rules, groups, counts, groupCounts, noSearch, total: rules.length, store, recordRepeatAtCeiling, searchNothing };
}
