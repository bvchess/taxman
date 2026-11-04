// The map of the site, in one place: the contents rail, the menu on small screens and the front page's figure of the
// chapters all draw from these lists.  The story's pages follow OUTLINE.md.  An entry without href is not yet
// written.

/** The story's first entry: the front page, which introduces it.  It has no number; the chapters keep theirs. */
export const INTRO = { title: 'Introduction', mark: 'i', href: 'index.html' };

export const STORY = [
  { n: 1, title: 'A game is a set', line: 'Which numbers you take decides the score; the order follows.', href: 'sets.html' },
  { n: 2, title: 'Walking up the chain', line: 'Most games are easy if you know an optimal game of the one before.', href: 'chain.html' },
  { n: 3, title: 'The top half', line: 'One greedy pass chooses the numbers above n/2.', href: 'top-half.html' },
  { n: 4, title: 'Frames', line: 'What is left splits into frames, and only the promotions are left to choose.', href: 'frames.html' },
  { n: 5, title: 'The search', line: 'A search tries promotion sets, and a bound proves when nothing better is left.', href: 'search.html' },
  { n: 6, title: 'Trust', line: 'Every score is checked, and every score says what it rests on.', href: 'trust.html' },
  { n: 7, title: 'The results', line: 'The games, how each was proven, and what they show.', href: 'results.html' },
];

/** The findings along the way, the story chapter each belongs beside, and, once written, where it is.  The navigation
 *  lists them as one entry.  The title and line are written once, here: the cards of the front page and of "Along the
 *  way" and the summary of each chapter's aside all draw from them.  A line is HTML, and may hold the spans that a
 *  page fills with numbers. */
export const ALONG = [
  {
    title: 'More moves can score less', where: 'game 833', id: 'game-833', at: 1, href: 'sets.html#game-833',
    line: `The best game is not always the longest.  Every optimal game of 833 makes
      <span data-833="moves">362</span> moves, but some games of 833 make <span data-833="longestMoves">363</span>, and
      they score less.`,
  },
  {
    title: 'Three numbers of tax for one move', where: 'game 40', id: 'game-40', at: 1, href: 'sets.html#game-40',
    line: `A move usually gives the taxman one or two numbers, yet the best game can give three.  In game 40, an
      optimal game takes 30 while 6, 10 and 15 are all still in the pot.`,
  },
  {
    title: 'Easy from n alone', where: 'every game', id: 'cofactor-rule', at: 2, href: 'chain.html#cofactor-rule',
    line: `Whether a game needs a search can usually be told from n alone: divide out its largest prime factor and
      look at what is left.  Game 3,002 = 79 \u00d7 38 needed no search, since 38 is twice a prime; game 3,003 =
      13 \u00d7 231 needed one.`,
  },
  {
    title: 'Largest first still works', where: 'game 2873', id: 'game-2873', at: 3, href: 'top-half.html#game-2873',
    line: `The numbers above n/2 can be chosen by one simple rule: take them from the largest down, keeping each one
      that can still be played.  In game 2,873 the usual reason to trust such a rule fails, yet it still keeps the
      heaviest set, as it does in every game up to 100,000.`,
  },
  {
    title: 'Not always one best game', where: 'game 425', id: 'game-425', at: 4, href: 'frames.html#game-425',
    line: `The best set of moves is not always the only one.  Up to game 424 each game has one; game 425 has two,
      which differ in three numbers and score the same.`,
  },
  {
    title: 'A bound that is exact', where: 'games 1 to 77', id: 'exact-to-77', at: 6, href: 'trust.html#exact-to-77',
    line: `A quick calculation with odd and even numbers sets an upper limit on a game\u2019s best score.  In every
      game up to 77 the limit is the best score itself; game 78 is the first it misses, by one point.`,
  },
  {
    title: 'The player\u2019s share', where: 'every game', id: 'share', at: 7, href: 'results.html#share',
    line: `As n grows, the best game wins the player an almost steady share of the board, which Hensley proved lies
      between 1/2 and 35/48.  In game 5,060 it is <span data-share>62.9%</span>.`,
  },
];

/** The entry that follows the story in the navigation; one without href is not yet written. */
export const AFTER = [
  { title: 'Along the way', line: 'The smaller findings', href: 'along.html' },
];

/** The entries below the navigation's line: the reference pages. */
export const REFERENCE = [
  { title: 'Glossary', mark: 'Aa', line: 'The words of the site', href: 'glossary.html' },
  { title: 'Notation', mark: 'Σ', line: 'The marks, pills and symbols', href: 'notation.html' },
  { title: 'About', line: 'What this work adds', href: 'about.html' },
];

/** Where a chapter lives, or null if it is not yet written. */
export function hrefOf(entry) {
  return entry.href ?? null;
}
