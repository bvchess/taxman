// The map of the site, in one place: the contents rail, the menu on small screens and the front page's map all draw
// from these lists.  The main story's pages follow OUTLINE.md.  An entry without href is not yet written.

export const STORY = [
  { n: 1, title: 'The game', line: 'The rules, and a game to play.' },
  { n: 2, title: 'A game is a set', line: 'What can be played: the order matters less than the set.' },
  { n: 3, title: 'Walking up the chain', line: 'How each game is built from the ones before it.' },
  { n: 4, title: 'The top half', line: 'Which numbers up to n/2 to play.' },
  { n: 5, title: 'Frames', line: 'The board falls into frames, nearly independent small games.' },
  { n: 6, title: 'The search', line: 'How the solver searches the promotions, its bounds, and what it costs.' },
  { n: 7, title: 'Trust', line: 'Two programs that check each other.' },
  { n: 8, title: 'The results', line: 'The chain of optimal games, and what supports each score.' },
];

/** The prototype chapter that chapters 4 and 5 will be built from. */
export const SAMPLE = { title: 'Sample: from the board to the frames', href: 'frames.html' };

/** The findings along the way, and the story chapter each belongs beside.  The navigation lists them as one entry. */
export const ALONG = [
  { title: 'A longer game is not a better one', where: 'game 833', id: 'game-833', at: 2 },
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
