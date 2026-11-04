// The Taxman model for the explanatory website: the game, its frames, and playability.
//
// Plain functions on plain data, with no DOM and no dependencies, so the same file runs in a browser (as an ES
// module, served over http) and in Node.  The words follow docs/GLOSSARY.md; the theory behind the frame model is in
// docs/theory/frame-model.md, and the peel theorem is in docs/theory/playability.md.
//
// Lists of numbers are plain arrays, ascending unless a function says otherwise.  Game states are treated as
// immutable: no function here changes a state it is given.

const ascending = (a, b) => a - b;

// ---------------------------------------------------------------------------------------------------------------
// Number theory
// ---------------------------------------------------------------------------------------------------------------

// Remember each number's answer, and hand out a copy so that a caller can never change what is remembered.
function remembered(compute) {
  const answers = new Map();
  return (m) => {
    if (!answers.has(m)) answers.set(m, compute(m));
    return [...answers.get(m)];
  };
}

/** The distinct primes that divide m, ascending: 12 gives [2, 3].  The number 1 has none. */
export const primeFactors = remembered((m) => {
  const primes = [];
  let rest = m;
  for (let p = 2; p * p <= rest; p++) {
    if (rest % p !== 0) continue;
    primes.push(p);
    while (rest % p === 0) rest /= p;
  }
  if (rest > 1) primes.push(rest);
  return primes;
});

/** Omega(x): the number of prime factors of x counted with multiplicity, which is x's layer.  12 gives 3; 1 gives 0. */
export function omega(x) {
  let count = 0;
  let rest = x;
  for (let p = 2; p * p <= rest; p++) {
    while (rest % p === 0) {
      rest /= p;
      count++;
    }
  }
  return rest > 1 ? count + 1 : count;
}

/** The maximal factors of m, descending: m/p for each distinct prime p dividing m.  12 gives [6, 4]; a prime gives
 *  [1]; the number 1 gives []. */
export function maximalFactors(m) {
  return primeFactors(m).map((p) => m / p);
}

/** Every divisor of m below m, ascending: 12 gives [1, 2, 3, 4, 6].  The number 1 has none. */
export const properDivisors = remembered((m) => {
  const small = []; // the divisors up to the square root
  const large = []; // their partners m/d above it, in descending order
  for (let d = 1; d * d <= m; d++) {
    if (m % d !== 0) continue;
    small.push(d);
    if (d * d !== m) large.push(m / d);
  }
  return [...small, ...large.reverse()].filter((d) => d < m);
});

// ---------------------------------------------------------------------------------------------------------------
// The game
// ---------------------------------------------------------------------------------------------------------------

/** A new game on the numbers 1 to n.  The pot is a Set, kept in ascending order; player and taxman list what each has
 *  taken, in order; log has one entry per turn, { move, tax: [...] }, and a last { rest: [...] } once finished. */
export function newGame(n) {
  const pot = new Set();
  for (let m = 1; m <= n; m++) pot.add(m);
  return { n, pot, player: [], taxman: [], log: [] };
}

// A move must be in the pot and still have a proper divisor there.  (Every reachable pot also gives the same answer
// with the maximal factors alone, which is the lifting argument of docs/theory/playability.md.)
function isLegal(pot, m) {
  return pot.has(m) && properDivisors(m).some((d) => pot.has(d));
}

// Take move m from state, which it changes: the player scores m, and the taxman takes every proper divisor in the pot.
// play and replay both go through here, so there is one statement of the rule.
function applyMove(state, m) {
  if (!state.pot.has(m)) throw new Error(`illegal move ${m} in game ${state.n}: it is not in the pot`);
  if (!isLegal(state.pot, m)) {
    throw new Error(`illegal move ${m} in game ${state.n}: it has no proper divisor in the pot`);
  }
  const tax = properDivisors(m).filter((d) => state.pot.has(d));
  state.pot.delete(m);
  for (const d of tax) state.pot.delete(d);
  state.player.push(m);
  state.taxman.push(...tax);
  state.log.push({ move: m, tax });
}

function copyOf(state) {
  return {
    n: state.n,
    pot: new Set(state.pot),
    player: [...state.player],
    taxman: [...state.taxman],
    log: [...state.log],
  };
}

/** The moves open to the player now, ascending: the numbers in the pot that still have a proper divisor in it. */
export function legalMoves(state) {
  return [...state.pot].filter((m) => isLegal(state.pot, m)).sort(ascending);
}

/** The state after the player takes m.  Throws if m is not a legal move. */
export function play(state, m) {
  const next = copyOf(state);
  applyMove(next, m);
  return next;
}

/** True when no move is left.  The taxman then takes the rest (see finish). */
export function isOver(state) {
  return legalMoves(state).length === 0;
}

// Close state, which it changes: the taxman takes whatever is left in the pot.
function applyFinish(state) {
  if (!isOver(state)) throw new Error(`game ${state.n} is not over: ${legalMoves(state).length} moves are left`);
  const rest = [...state.pot].sort(ascending);
  state.pot.clear();
  state.taxman.push(...rest);
  state.log.push({ rest });
}

/** The state after the taxman takes the rest.  Throws if a move is still left, since the taxman takes the rest only
 *  when none is. */
export function finish(state) {
  const next = copyOf(state);
  applyFinish(next);
  return next;
}

/** The score of a state: the sum of the player's moves. */
export function score(state) {
  return state.player.reduce((total, m) => total + m, 0);
}

/** Play the moves in order on game n, then let the taxman take the rest.  Throws on an illegal move, and if a move is
 *  still left at the end. */
export function replay(n, moves) {
  const state = newGame(n);
  for (const m of moves) applyMove(state, m);
  applyFinish(state);
  return state;
}

// ---------------------------------------------------------------------------------------------------------------
// The frame model
// ---------------------------------------------------------------------------------------------------------------

/** The sources of game n: the numbers above n/2, ascending.  Game 21 has 11 to 21. */
export function sources(n) {
  const list = [];
  for (let m = Math.floor(n / 2) + 1; m <= n; m++) list.push(m);
  return list;
}

/** The frames of game n: the connected components of the graph that joins each source to its maximal factors.
 *  Each is { layer, sources, factors }, both lists ascending.  Frames are ordered by layer, then by smallest source.
 *  Game 21 has four: layer 1 (11, 13, 17, 19 with 1), layer 2 (14, 15, 21 with 2, 3, 5, 7), and so on. */
export function frames(n) {
  const factorsOf = new Map(); // source -> its maximal factors
  const sourcesOf = new Map(); // factor -> the sources it is a maximal factor of
  for (const s of sources(n)) {
    factorsOf.set(s, maximalFactors(s));
    for (const f of factorsOf.get(s)) {
      if (!sourcesOf.has(f)) sourcesOf.set(f, []);
      sourcesOf.get(f).push(s);
    }
  }

  const found = [];
  const seen = new Set();
  for (const start of sources(n)) {
    if (seen.has(start)) continue;
    // Collect the component of start by walking source -> factor -> source until nothing new turns up.
    const frameSources = [];
    const frameFactors = new Set();
    const todo = [start];
    seen.add(start);
    while (todo.length > 0) {
      const s = todo.pop();
      frameSources.push(s);
      for (const f of factorsOf.get(s)) {
        frameFactors.add(f);
        for (const other of sourcesOf.get(f)) {
          if (seen.has(other)) continue;
          seen.add(other);
          todo.push(other);
        }
      }
    }
    frameSources.sort(ascending);
    found.push({ layer: omega(start), sources: frameSources, factors: [...frameFactors].sort(ascending) });
  }
  return found.sort((a, b) => a.layer - b.layer || a.sources[0] - b.sources[0]);
}

/** Decide playability of a set of numbers by peel, on the set's own instance (docs/theory/playability.md).
 *  The members are the set, the pool is their maximal factors outside it, and each member lists its maximal factors in
 *  the pool.  Peel pairs a member with a factor, one pair a step, in either of two ways:
 *    front: a member that lists exactly one live factor takes that factor;
 *    back:  a live factor that exactly one remaining member lists goes to that member.
 *  The set is playable exactly when every member gets paired, whatever the order of steps (the peel theorem).
 *  Returns { playable, steps: [{ kind: 'front' | 'back', member, factor }], stuck }, where stuck lists the members left
 *  when peel stops (ascending; empty if playable).  Peel stops early, with the set unplayable, when a member is left
 *  with no live factor.  Fronts go before backs, and each in the order it becomes available, so a run is repeatable. */
export function peel(set) {
  const members = [...new Set(set)].sort(ascending);
  const inSet = new Set(members);
  const list = new Map(); // member -> the live factors it lists
  const listedBy = new Map(); // live factor -> the remaining members that list it
  for (const x of members) {
    const factors = maximalFactors(x).filter((f) => !inSet.has(f));
    list.set(x, new Set(factors));
    for (const f of factors) {
      if (!listedBy.has(f)) listedBy.set(f, new Set());
      listedBy.get(f).add(x);
    }
  }

  const remaining = new Set(members);
  const steps = [];
  // Work lists of candidate moves.  An entry can go stale before it is used, so it is checked again when taken.
  const fronts = members.filter((x) => list.get(x).size === 1);
  const backs = [...listedBy.keys()].filter((f) => listedBy.get(f).size === 1).sort(ascending);
  let starved = members.some((x) => list.get(x).size === 0);

  // Pair member x with factor f, and update what the others list.
  function pair(kind, x, f) {
    steps.push({ kind, member: x, factor: f });
    remaining.delete(x);
    for (const g of list.get(x)) {
      listedBy.get(g).delete(x);
      if (g !== f && listedBy.get(g).size === 1) backs.push(g);
    }
    for (const y of listedBy.get(f)) {
      // Only after a front move are there others; f leaves their lists.
      list.get(y).delete(f);
      if (list.get(y).size === 1) fronts.push(y);
      if (list.get(y).size === 0) starved = true;
    }
    listedBy.delete(f);
  }

  while (remaining.size > 0 && !starved) {
    if (fronts.length > 0) {
      const x = fronts.shift();
      if (remaining.has(x) && list.get(x).size === 1) pair('front', x, [...list.get(x)][0]);
    } else if (backs.length > 0) {
      const f = backs.shift();
      if (listedBy.has(f) && listedBy.get(f).size === 1) pair('back', [...listedBy.get(f)][0], f);
    } else {
      break;
    }
  }
  return { playable: remaining.size === 0, steps, stuck: [...remaining].sort(ascending) };
}

/** True when every number of the set can be played as a move, in some order.  The set's own instance decides it. */
export function isPlayable(set) {
  return peel(set).playable;
}

/** U*: the sources that descending greedy keeps in game n, ascending.  It scans the sources from the largest down and
 *  keeps each one that is playable together with those kept before it.  Game 21 gives 12, 14, 15, 16, 18, 19, 20 and
 *  21. */
export function descendingGreedy(n) {
  const kept = [];
  for (const s of sources(n).reverse()) {
    if (isPlayable([...kept, s])) kept.push(s);
  }
  return kept.sort(ascending);
}

/** The frames of game n with what the frame model makes of each.  Each is a frame (layer, sources, factors) and also:
 *    kept      its sources in U*, the moves the model fixes;
 *    rejected  its other sources, which descending greedy drops;
 *    freedoms  its factors minus its kept sources, the factors it can spare for promotions;
 *    closed    true when freedoms is 0, and the frame takes no part in the search. */
export function frameModel(n) {
  const inUStar = new Set(descendingGreedy(n));
  return frames(n).map((frame) => {
    const kept = frame.sources.filter((s) => inUStar.has(s));
    const rejected = frame.sources.filter((s) => !inUStar.has(s));
    const freedoms = frame.factors.length - kept.length;
    return { ...frame, kept, rejected, freedoms, closed: freedoms === 0 };
  });
}
