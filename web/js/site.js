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
 *  lists them as one entry. */
export const ALONG = [
  { title: 'Three factors of tax', where: 'game 40', id: 'game-40', at: 1, href: 'sets.html#game-40' },
  { title: 'More moves, fewer points', where: 'game 833', id: 'game-833', at: 1, href: 'sets.html#game-833' },
  { title: 'The cofactor rule', where: 'every game', id: 'cofactor-rule', at: 2, href: 'chain.html#cofactor-rule' },
  { title: 'The greedy that shouldn\'t work', where: 'game 2873', id: 'game-2873', at: 3, href: 'top-half.html#game-2873' },
  { title: 'Two optimal sets', where: 'game 425', id: 'game-425', at: 4, href: 'frames.html#game-425' },
  { title: 'A bound that is exact', where: 'games 1 to 77', id: 'exact-to-77', at: 6, href: 'trust.html#exact-to-77' },
  { title: 'The player\'s share', where: 'every game', id: 'share', at: 7, href: 'results.html#share' },
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
