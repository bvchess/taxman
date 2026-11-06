// The map of the site, in one place: the contents rail, the menu on small screens and the front page's figure of the
// chapters all draw from these lists, and so do the links at the foot of each page, which lead from each page of the
// map to the next in this sequence: INTRO, then STORY.  The reference pages stand on their own: the rail lists them,
// and no link leads to or from them.  A line is a sentence, with its period.  The story's pages
// follow OUTLINE.md.  An entry without href is not yet written.

/** The story's first entry: the front page, which introduces it.  It has no number; the chapters keep theirs. */
export const INTRO = { title: 'Introduction', mark: 'i', href: 'index.html' };

export const STORY = [
  { n: 1, title: 'A game is a set', line: 'Which numbers you take decides the score; the order follows.', href: 'sets.html' },
  { n: 2, title: 'Walking up the chain', line: 'Most games are easy if you know an optimal game of the one before.', href: 'chain.html' },
  { n: 3, title: 'The top half', line: 'One greedy pass chooses the numbers above n/2.', href: 'top-half.html' },
  { n: 4, title: 'Frames', line: 'What is left splits into frames, and only the promotions are left to choose.', href: 'frames.html' },
  { n: 5, title: 'The search', line: 'A search tries promotion sets, and a bound proves when nothing better is left.', href: 'search.html' },
  { n: 6, title: 'Trust', line: 'Every score is checked, and every score says what it rests on.', href: 'trust.html' },
  {
    n: 7, title: 'The results', line: 'The best scores, how each was proven, and what they show.', href: 'results.html',
  },
  { n: 8, title: 'In Summary', line: 'The optimal games in brief, and the whole method on game 21.', href: 'summary.html' },
];

/** The findings along the way, the story chapter each belongs beside, and, once written, where it is.  The navigation
 *  lists them as one entry.  The title and line are written once, here: the cards of the front page and of "Along the
 *  way" and the summary of each chapter's aside all draw from them.  A line is HTML, and may hold the spans that a
 *  page fills with numbers. */
export const ALONG = [
  {
    title: 'More moves isn\'t always better', where: 'game 833', id: 'game-833', at: 1, href: 'sets.html#game-833',
    line: `The best game is not always the one with the most moves.  In game 833, every optimal game makes
      <span data-833="moves">362</span> moves, but some games make <span data-833="longestMoves">363</span>, and they
      score less.`,
  },
  {
    title: 'Three numbers for tax on one move isn\'t impossible', where: 'game 40', id: 'game-40', at: 1, href: 'sets.html#game-40',
    line: `In an optimal game a move usually gives the taxman one or two numbers, but there are places where an optimal game can give three.  In game 40, an
      optimal game can claim 30 while 6, 10 and 15 are all still in the pot.`,
  },
  {
    title: 'One search, eight games', where: 'games 2,512 to 2,519', id: 'one-search', at: 2, href: 'chain.html#one-search',
    line: `A game often poses exactly the search problem of an earlier game and so needs no search of its own.  Game
      2,512 poses a new one, and games 2,513 to 2,519 all pose it again: one search serves eight games, more than any
      other in the chain.`,
  },
  {
    title: 'Not always one best game', where: 'game 425', id: 'game-425', at: 4, href: 'frames.html#game-425',
    line: `The optimal set of moves is not always unique.  Up to game 424 each game has one unique set of optimal moves,
     but game 425 has two, which differ in three numbers and score the same.`,
  },
  {
    title: 'The player\u2019s share', where: 'every game', id: 'share', at: 7, href: 'results.html#share',
    line: `As n grows, the best game wins the player an almost steady share of the board, which Hensley proved lies
      between 50% and about 73%.  It seems to converge to ~<span data-share>62.9%</span>.`,
  },
];

/** The entries below the navigation's line: the reference pages. */
export const REFERENCE = [
  { title: 'Glossary', mark: 'Aa', line: 'The words of the site.', href: 'glossary.html' },
  { title: 'Notation', mark: 'Σ', line: 'The marks, pills and symbols.', href: 'notation.html' },
  { title: 'About', line: 'What this work adds.', href: 'about.html' },
];

/** Where a chapter lives, or null if it is not yet written. */
export function hrefOf(entry) {
  return entry.href ?? null;
}
