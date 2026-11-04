// The map of the site, in one place: the contents rail, the menu on small screens and the front page's figure of the
// chapters all draw from these lists.  The main story's pages follow OUTLINE.md.  An entry without href is not yet
// written.

export const STORY = [
  { n: 1, title: 'The game', line: 'Easy to play well, hard to play perfectly.' },
  { n: 2, title: 'A game is a set', line: 'Which numbers you take decides the score; the order follows.', href: 'sets.html' },
  { n: 3, title: 'Walking up the chain', line: 'Most games are easy if you know an optimal game of the one before.' },
  { n: 4, title: 'The top half', line: 'One greedy pass chooses the numbers above n/2.' },
  { n: 5, title: 'Frames', line: 'What is left splits into frames, and only the promotions are left to choose.' },
  { n: 6, title: 'The search', line: 'A search tries promotion sets, and a bound proves when nothing better is left.' },
  { n: 7, title: 'Trust', line: 'Every score is checked, and every score says what it rests on.' },
  { n: 8, title: 'The results', line: 'The games, how each was proven, and what they show.' },
];

/** The prototype chapter that chapters 4 and 5 will be built from. */
export const SAMPLE = { title: 'Sample: from the board to the frames', href: 'frames.html' };

/** The findings along the way, the story chapter each belongs beside, and, once written, where it is.  The navigation
 *  lists them as one entry. */
export const ALONG = [
  { title: 'Three factors of tax', where: 'game 40', id: 'game-40', at: 2, href: 'sets.html#game-40' },
  { title: 'More moves, fewer points', where: 'game 833', id: 'game-833', at: 2, href: 'sets.html#game-833' },
  { title: 'Games with several optimal sets', where: 'from game 425 on', at: 8 },
];

/** The entries after the story in the navigation, not yet written. */
export const AFTER = [
  { title: 'Along the way', line: 'The smaller findings' },
  { title: 'About', line: 'What this work adds' },
];

/** Where a chapter lives, or null if it is not yet written. */
export function hrefOf(entry) {
  return entry.href ?? null;
}
