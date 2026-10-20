# Glossary

The words of the project, each with the one name the docs use for it.  The sections follow the order in which the
ideas build on each other: the game, the frame model, the search, the bounds, results and trust, the programs, and the
labels of plans and statements.  A word that means more than one thing is marked **(overloaded)**, with each meaning.

Some words are used inside one part of the code only, and are defined there:

* setup's (`unzip`, claimed factors, sinks): the module comment of `solver/frame/src/frames/mod.rs`;
* the peel inside a frame, `reduce` and the home frame: `solver/frame/src/model/`;
* the bounds' internals (potentials, weight splits, contraction, refinement, refits):
  [solver/docs/design/search.md](../solver/docs/design/search.md) and
  [solver/docs/design/gf2-bound.md](../solver/docs/design/gf2-bound.md);
* the engine's (units, splits, steals, spills, pauses): [solver/docs/design/engine.md,
  "Engine"](../solver/docs/design/engine.md#engine) and the module comment of `solver/frame/src/engine.rs`;
* the fields of the progress line: [the frame solver's reference, section
  3](../solver/docs/REFERENCE.md#3-reading-the-output).

The wiki's words map onto these in [solver/docs/design/frame-model.md, "The Java solver and the
frame solver"](../solver/docs/design/frame-model.md#the-java-solver-and-the-frame-solver).  The old words of the
notebook's dated entries map onto these in the notebook's ["Terms changed"](../solver/notebook/terms-changed.md).

## The game

* **game** **(overloaded)**: (1) the instance.  *Game n*, or *the game of n*, is Taxman played on the numbers 1 to n:
  game 21 is Taxman on 1 to 21.  (2) A play of it: a legal sequence of moves, as in "an optimal game of 21".  Where a
  run has solved game n, "the game of n" also names the optimal game it recorded.  Say "game n", never "board size".
* **board**: the numbers 1 to n of game n.  On the board of 21 the sources are 11 to 21.  In the code, `board::Board`
  is a board analysed for the search: its tables, frames, candidates and `in_the_bag`.
* **pot**: the numbers still in play during a game.  It starts as the whole board.
* **move**: a number the player takes, which must still have a proper divisor in the pot.  The player scores it.  The
  prover's README and the heuristics say *pick*.
* **tax**: the divisors of a move still in the pot, which the taxman takes with it.  When no move is left, the taxman
  takes whatever remains.
* **score**: the sum of a game's moves.  **opt(n)**: the best score of game n.  **optimal game**: a game that scores
  opt(n).  Game 6 has the optimal game 5, 4, 6 (the taxman takes 1, then 2, then 3), so opt(6) = 15.
* **Taxman sequence**: opt(1), opt(2), opt(3), ..., which is [OEIS A019312](https://oeis.org/A019312).
* **optimal set**: the set of moves of an optimal game.  Many orders play one set.  From game 425 on, some games have
  two or three optimal sets ([solver/noframe/docs/uniqueness.md](../solver/noframe/docs/uniqueness.md)).
* **maximal factor**: of a number m, the quotient m/p for a prime p that divides m.  12 has the maximal factors 6 and
  4.  Its inverse is the **maximal composite**: on the board of 85, 8 has the maximal composites 16, 24, 40 and 56.
  The **maximal factor graph** joins each number to its maximal factors.  A move is legal exactly when one of its
  maximal factors is still in the pot: if some proper divisor of the move is in the pot, so is a maximal factor.  This
  is the *lifting argument* (`solver/frame/src/numtheory.rs`; [THEORY.md](THEORY.md), Theorem 1, which proves it).  A
  maximal factor still in the pot *pays for* the move: it is among the divisors the taxman takes with it.  The taxman
  takes every divisor of a move, but the frame model needs only the maximal factors, and a factor pays for one move.
* **playable**: a set of numbers is playable when all of its members can be played as moves, in some order.
  **peel**: the procedure that decides it.  It repeatedly drops a member that has a factor no other member has, and the
  set is playable exactly when nothing is left ([THEORY.md](THEORY.md), Theorems 1 to 3).  Inside a frame the same
  procedure decides whether the frame is winnable.
* **ceiling**: n + opt(n − 1), which no game of n can exceed: deleting the move n from an optimal game of n leaves a
  game of n − 1.  The solver uses n + score(n − 1), with the recorded score of n − 1.  A game *at its ceiling* scores
  exactly that, and needs no search to prove (`results::CeilingProof`; the bound is Lemma L2 of
  [theory/ledger.md](theory/ledger.md), and Theorem 9 gives the optimal sets there).  As a bound on the promotion sum,
  the ceiling is n + score(n − 1) − `in_the_bag`, which the results lines and a tactic call `cap`: the proof `cap`, the
  fields `reached_cap` and `setup.cap`, the tactic `prev_cap`.
* **prime formula**: for a prime n, opt(n) = n + opt(n − 1) − q, with q the largest prime below n.  The optimal game
  of n is that of n − 1 with its first move, q, replaced by n.  For game 7: 7 + 15 − 5 = 17.  The tactic is
  `prime_shortcut`, the proof `prime_formula`, and the proof of the formula is at `accelerate::prime_shortcut`.

## The frame model

* **source**: a number above n/2.  It divides nothing else on the board, so it is never tax: it is a move or nothing,
  and a move unless setup finds it impossible.  On the board of 21 the sources are 11 to 21.  A set's **upper part** is
  its members above n/2, which are sources.  The frame solver's docs and code, and THEORY.md, never say "upper half" for
  them; the heuristics do.
* **factor**: a number up to n/2.  It is tax for a larger number, and sometimes a move itself.  Not "lower half".
* **frame**: a group of sources that share factors, with all the factors they can be paid with.  On the board of 21, one
  frame has the sources 14, 15 and 21 and the factors 2, 3, 5 and 7.  In the code a frame is `frames::Frame` as setup
  forms it and `model::Frame` as the search takes it; `frames::BoardFrames` holds all of a board's, and
  `frames::SearchFrames` those the search works on.
* **in front of**, **behind**: frame X is in front of frame Y, and Y is behind X, when a number of Y has a maximal
  factor in X.  Y's numbers are then multiples of X's.  Frames in front are played first, so frame 0 is the front.  Not
  "upstream" or "downstream".
* **Taxman Mini**: the game a frame plays.  It has a set of moves and a set of factors.  Each move must still have one
  of its maximal factors in the frame when it is taken, and taking it removes all of its factors still in the frame.
* **winnable**: a frame is winnable when some order takes every move.  Peel decides it.
* **setup**: what the frame solver does to a board before it searches.  It forms the frames, simplifies them, numbers
  and merges them, and finds the candidates and `in_the_bag`.
* **mandatory move**: a move that setup settles (`unzip`).  A source with one usable factor must take it, and a factor
  that one source alone wants goes to that source.  On the board of 21, 15 must take 5 and 20 must take 10.  Never
  "forced move": *forced* is the search's word.
* **impossible source**: a source that cannot be played together with the larger sources already kept.  Setup finds
  them greedily from the largest source down (`removeImpossible`) and drops them from the frame's moves.  On the board
  of 21 the sources 11, 13 and 17 are impossible: their frame's only factor, 1, pays for 19.
* **U\***: the sources that setup keeps.  It is the heaviest playable set of sources wherever conjecture (★) holds,
  which is certified for every game to 100,000 ([solver/noframe/docs/star.md, "Conjecture (★) at
  scale"](../solver/noframe/docs/star.md#conjecture--at-scale)).  Its members with two prime factors are U\*_2, and
  those with three or more U\*_{≥3}.
* **freedoms**: a frame's factors minus its moves, the number of factors it can spare.  A promotion into or out of the
  frame uses one up, and none may go below zero.  A frame with none is **closed** and takes no part in the search.
* **layer**: the number of prime factors of a number, counted with multiplicity: 12 = 2 · 2 · 3 is in layer 3.  A
  frame's layer is its sources'.  The frames in front of a frame lie one layer lower.
* **frame number**: setup numbers the frames that have freedoms by layer, from the front.  If L is the lowest layer of a
  frame with freedoms, frame h is the frames with freedoms in layer L + h.  Frames with the same number are merged into
  one, so that a promotion always has one frame to go to.  The numbering is **clean** (statement N of THEORY.md): every
  frame with freedoms directly in front of frame h is frame h − 1.  The numbering by layer is clean by construction.  On
  every board the solver supports it equals the numbering breadth first from the front, the Java's: frame 0 is every
  frame with freedoms that has no other in front of it, and frame k + 1 every frame directly behind frame k.
* **promote**, **promotion**: to play a factor as a move.  Promoting a factor of frame h takes it out of the frame's
  factors and makes it a move of frame h − 1, one frame forward.  It is legal when both frames stay winnable.  The
  maximal factors that pay for the promoted number lie in the frames directly in front of its own, and the numbering
  puts those that have freedoms in frame h − 1; that nothing further forward is needed is a theorem
  ([theory/frame-model.md](theory/frame-model.md)).  After promotions the frames are *reduced*: a move left with one
  live factor is paid by it, and the pair is dropped, repeatedly.
* **candidate** **(overloaded)**: (1) a number that setup says could be promoted (`SearchFrames::candidates`,
  `BoardFrames::promotion_candidates`).  Game 21 has 2 candidates, game 85 has 10, and game 1100 has 273.  (2) In a
  search node, `cands`: the candidates the node's loop still has to try.  (3) A role in the matching bound's graph: a
  number that may be matched as a promoted move, pay for one behind it, or stay free.
* **root**: the frames as setup leaves them, before any promotion.
* **promotion set**: a set of candidates that can be promoted together from the root, keeping every frame winnable and
  its freedoms at least 0.  *Promotable* is its adjective: a promotable set is a promotion set.  The score of a game of
  the model is `in_the_bag` plus the sum of its promotion set.  On the board of 21, {9} and {4} are promotion sets.
* **in the bag** (`in_the_bag`): what the game with no promotions scores.  Game 21 has `in_the_bag` = 135.
* **max_total**: the most promotions the frames' freedoms allow.
* **search problem**, **Π**: what setup gives the search, less what only changes how the search runs: the frames, the
  candidates, `max_total` and the factor graph among their numbers.  `search::Problem` writes it out canonically, and
  `setup.digest` in a results line is a 64-bit digest of it.  Π_n is the root problem of game n
  ([solver/docs/design/same-search.md](../solver/docs/design/same-search.md)).
* **P\***: the best promotion sum, the largest sum of a promotion set.  The best promotion sum of a problem Π is P\*(Π),
  and P\* alone means P\*(Π_n).  The frame solver's answer for game n is `in_the_bag` + P\*.  Game 21 has P\* = 9.
* **the frame model**: this decomposition of the game into frames and promotions.  It is exact given statement U: then
  `in_the_bag` + P\* = opt(n) ([theory/frame-model.md](theory/frame-model.md); [solver/docs/design/frame-model.md, “What
  "optimal" means”](../solver/docs/design/frame-model.md#what-optimal-means)).  `model::FrameGraph` is the immutable
  factor graph and home frames, not "the model".
* **statement U**: some optimal game plays every member of U\*_{≥3}.  The frame solver's answers rest on it where their
  proofs use the frame model: an exhausted search, a trivial game or a repeat assumes U at n, and a proof by the ceiling
  or by the prime formula does not ([solver/docs/design/frame-model.md, “What "optimal"
  means”](../solver/docs/design/frame-model.md#what-optimal-means)).  U is open in general, and proven at every n the
  prover has proven.  Past those games `taxman certify` checks it game by game ([certificate of U](#results-and-trust)).
* **extended problem**, **Π⁺(O)**: the frame model with a set O of members of U\* made optional
  ([THEORY.md](THEORY.md), Theorem M⁺; `frames::build_extended`).  The certificates of U search it.

## The search

* **tactic**: a switchable piece of the frame solver's algorithm (`--tactic NAME=VALUE`).  Each is graded *proven* or
  *sound* in [solver/docs/design/tactics.md](../solver/docs/design/tactics.md), so switching one changes how fast an
  answer comes, never the answer.
* **node**, **step**: a search node is a set of promotions made from the root, plus the candidates left to it.  The
  `nodes` that results and progress lines count are *steps*, one for each candidate a node's loop tries.
* **completion**: of a node, a promotion set that extends the node's promotions with some of its candidates.  Its
  **best completion** is the heaviest.  At the root the best completion is worth P\*.
* **incumbent**: the best promotion set found so far, shared by all threads.  A node's **need** is what its
  completions must reach to beat the incumbent: the incumbent's sum, less the node's own, plus one.
* **greedy**: promote every candidate that can be, from the largest down.  The search starts from the best of the greedy
  set, the seed and a rounding of the root's matching ([solver/docs/design/search.md, "The starting incumbent, the
  ceiling and the seed"](../solver/docs/design/search.md#the-starting-incumbent-the-ceiling-and-the-seed)).
* **seed**: a promotion set that a search starts from: the promotions of the game of n − 1, or the game of a lower
  bound that the bounds store holds.
* **conflict**: a subset of a promotion set that cannot be promoted together from a node.  Every completion leaves out
  one of its members.
* **nogood**: a conflict of the root, learned from a conflict found at a node.  It holds at every node, on any thread
  and any path.  Each searcher keeps a table of them and forgets it every 50,000 of its steps (tactic
  `forget_nogoods`).
* **forced member**: a member of a node's heaviest set whose removal alone brings the bound below the node's need.
  Every completion that beats the incumbent promotes it, so with the tactic `fix_forced` the child promotes all of
  them together.
* **strong branching**: how a node chooses the candidate to promote next: the member of the conflict whose removal
  lowers the bound the most.  With the GF(2) bound it is exact for each member, read off the exchange graph of the
  heaviest set where that certifies it and solved where not.
* **prime exchange**, **symmetric pair**: exchanging two primes p < q in every number.  The pair is symmetric when the
  exchange maps the root's frames and candidates onto themselves.  It then maps promotion sets to promotion sets
  worth (q − p)(c_p − c_q) more, where c_r is the sum of a set's cofactors x/r.  So every heaviest set keeps c_p ≤ c_q
  for every symmetric pair ([THEORY.md](THEORY.md), Theorem 13).  With the tactic `symmetry` the search cuts a node
  none of whose completions can keep them, and drops the candidates that no completion keeping them holds.
* **window** **(overloaded)**: (1) the games of a range in flight at once (`--window K`, `solver::window`; [the frame
  solver's README, "Games in flight"](../solver/frame/README.md#games-in-flight)).  A search started before the game
  before it is proven is *speculative*; it is seeded from that game's *provisional game*, its best so far.  A
  speculative search may be *deferred*, not started while the ceiling its provisional game gives would settle it, or
  *suspended*, its workers taken for the oldest game and its units kept.  (2) The eight games before game n, among
  which the frame solver looks for one whose search n repeats (`SAME_SEARCH_WINDOW`).  (3) The probe's windows: a
  checkpoint resumed for 50,000 steps ([solver/docs/design/performance.md](../solver/docs/design/performance.md)).
* **checkpoint**: a periodic pause in which all workers stop and the engine writes the unexplored work, the incumbent
  and statistics to `nNNNN.ckpt`.  Its **header** identifies the search: n, the set width, digests of the candidates
  and frames, `max_total` and the tactics.
* **the wall**: the first game whose own search takes longer than a set number of hours.  `taxman solve --game-hours
  H` ends a range there.  The project's limit (2026-10-16) is 24 hours of the game's own search, not time spent waiting
  behind another game, on one `c8g.16xlarge` with the run's build and settings: `--game-hours 24`, the hours the game
  has been the oldest of the window with its search running.

## Bounds

A bound below is an upper bound on what a set of promotions or moves can score, unless it says otherwise.

* **the gap**: how far an upper bound lies above the best completion, in promotion units (equal to score units, since
  `in_the_bag` is fixed).  Unqualified, *the gap* is the root's: the root's GF(2) bound minus P\*.  The searched
  games of 2176 to 3312 have root gaps from 341 to 1,443, and a search's cost grows with its root gap ([the gap past
  2175](../solver/notebook/the-gap-past-2175-2026-10/README.md)).  The tools also measure three related gaps, each named
  where it is used:
  * a node's gap (`examples/tune.rs`, `wall choice`): the node's bound less its need, how far the bound must fall to
    cut the node;
  * the standalone root gap (`wall landscape`): the root's bound less the search's start incumbent, for a game solved
    with no game of n − 1;
  * a game's gap in the ledger: its best upper bound less its best lower bound.

  The prover's *parity gap* is the same idea on the whole board: the root's parity bound less opt(n), in points.
* **matching bound** (`matchbound`; the tactic `gf2=off`): relaxes winnability to matchability.  It is the largest sum
  of promotions such that every move and every promoted number gets a distinct maximal factor in its frame.  It is a
  maximum-weight bipartite matching, kept along the search path.
* **GF(2) bound** (`gf2`, the default node bound): the heaviest set that is independent in two matroids over GF(2), one
  for the even frames and one for the odd ones ([THEORY.md](THEORY.md), Theorem 11; [solver/docs/design/gf2-bound.md,
  "The GF(2) bound"](../solver/docs/design/gf2-bound.md#the-gf2-bound)).
* **the Lagrangian bound over the nogoods**: an inner bound, either the matching bound or the GF(2) bound, with each
  candidate's weight lowered by multipliers on the live nogoods it is in.  Any multipliers give a valid bound.  Name the
  inner bound when it matters.  It comes in three forms:
  * the tactic `lagrange`: the matching bound inside, with multipliers that each searcher fits at the root to the
    nogoods it has learned ([design/search.md](../solver/docs/design/search.md#the-lagrangian-bound-over-the-nogoods));
  * refitted at a node: the GF(2) bound inside, with multipliers fitted again at the node (`wall choice`).  The
    notebook's entries call this form the "strong bound";
  * in the bounds store: the GF(2) bound inside, over nogoods each checked to fail to promote (the rule `nogoods`).

  The prover has a Lagrangian bound over its own nogoods, with the Franklín–Moniot matching inside.
* **the multi-field bound**: the bound of the agenda's avenue A1.  Every promotion set is independent in the frame
  matroids over several fields and signings: GF(2), GF(3) with 0/1 entries, and GF(3) with signs.  The bound is the
  linear program over all of them, computed by Lagrangian decomposition ([THEORY.md](THEORY.md), Theorems 14 and 15;
  [the fields study](../solver/notebook/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md)).  The
  agenda's A1 measured it and is closed ([solver/docs/AGENDA.md, "Ranked
  avenues"](../solver/docs/AGENDA.md#3-ranked-avenues)).  The notebook's entries call it the "fields bound" or the "A1
  bound".
* **dual bound**: what a search that a limit stopped proves from above.  It is the largest of the incumbent and, over
  the open units of the final checkpoint, each node's promotions plus its GF(2) bound (`search::dual_bound`;
  [THEORY.md](THEORY.md), Lemma D).  A search stopped before its first node has the root's bound as its dual bound.
* **floor**: a search from a floor F starts from an incumbent worth F that has no set.  If it ends having found
  nothing, no promotion set weighs more than F ([THEORY.md](THEORY.md), Lemma F).
* **Franklín–Moniot bound**: the maximum-weight matching relaxation of the whole game, an upper bound on opt(n), from
  [Franklín and Moniot (2023)](https://arxiv.org/abs/2211.00461).  `solver/tools/check_results.py` compares each score
  with it where `heuristics/results` has the bound.
* **parity bound**: the prover's bound.  Every playable set is independent in two matroids over GF(2), split by the
  parity of the number of prime factors, and the bound is their heaviest common independent set ([THEORY.md](THEORY.md),
  Theorem 10).
* **barred bound**: the parity bound with one member u of U\* barred.  The prover *certifies* u when the barred bound is
  at most the recorded score: then every set of moves that beats the score, and obeys Theorem B, plays u.  The members
  it does not certify are *open* ([solver/noframe/docs/proofs.md, "The upper part, game by
  game"](../solver/noframe/docs/proofs.md#the-upper-part-game-by-game)).
* **sacrifice bound** (`src/sacrifice.rs`): an upper bound on opt(n) from games on boards with holes.  The frame solver
  does not use it.

## Results and trust

* **chain**: games each built from the one before (n = 1, 2, 3, ...).  A game's search starts from the previous game's
  promotions and stops at its ceiling.  A game solved with no game of n − 1 is **standalone**.
* **the committed chain**: the chain in the file `results/strict.jsonl`, from n = 1.  Every game in it is proven within
  the frame model.  [results/README.md](../results/README.md#strictjsonl) gives its range.  The **end of the chain** is
  its last game.
* **repeat**: a game that poses the same search problem as an earlier game, which settled it.  The frame solver
  concludes it with the earlier game's promotions instead of searching.  `same_search` is the name of the tactic and of
  the proof; the method of the line is `repeat`.
* **proof** (kind): how a completed game's optimality is established: `trivial`, `exhausted`, `cap`, `prime_formula`,
  `same_search` or `known` ([the frame solver's reference, "Proof
  kinds"](../solver/docs/REFERENCE.md#proof-kinds)).
* **assumes**: what a proof rests on beyond the frame model, inherited down the chain.  A game of n − 1, or of n, taken
  from `optimal.json` is assumed optimal.  A bound of the bounds store assumes statement U at the games it rests on.
* **known game**, **known source**: a game that `taxman solve --use-prev` takes instead of searching for it, from
  `optimal.json`, the committed chain, or a game that the bounds store has proven (`src/known.rs`).
* **bounds store**: `results/bounds.jsonl`, the bounds on opt(n) that the ledger finds, one line each, kept for good
  ([results/README.md](../results/README.md#boundsjsonl); `src/store.rs`).  A game is **proven** there when its best
  lower bound equals its best upper bound.
* **the ledger**: the example program `ledger` (`solver/frame/examples/ledger/`).  It works on a whole stretch of games
  at once, raising lower bounds and lowering upper bounds until they meet.
* **the sweep** (`solver/tools/cloud/sweep.sh`): a limited search of every game of a long stretch, holes allowed.  Its
  file, `explore.jsonl`, is not a chain.
* **certificate of U**: a line of `taxman certify` that proves statement U at one game.  The extended problem with the
  game's open members optional, searched from the recorded game, ends `certified` when nothing beats the recorded
  score ([solver/docs/design/certificates.md](../solver/docs/design/certificates.md)).
* **optimal.json**: the table of optimal games, n ≤ 1300, built into the binary
  ([results/README.md](../results/README.md#optimaljson) says where its games came from).  **optimal ok**: a score
  equals the table's.  That is not a proof, and above the table's last game there is no table to match; a score that
  differs is a hard error.
* **referee**: code that replays one game and checks it: legal, over, opening with the largest prime, scoring what is
  recorded.  `game::replay` and `game::replay_finished` are the frame solver's.  A game that fails after it was found
  is **rejected**, written to `<results>.rejected.jsonl`.
* **verifier**: a program that checks a whole results file or bounds file without changing it.  Results files have two,
  `taxman verify-results` and `solver/tools/check_results.py`; the bounds store has `ledger verify-bounds` and
  `solver/tools/check_bounds.py`; certificates have `taxman verify-certificates`.  The two verifiers of a kind of file
  share no code.  Not "checker".
* **results line**: one line of a results file, the JSON object that `taxman solve --out` writes for a game, in schema
  5 ([the frame solver's reference](../solver/docs/REFERENCE.md#a-results-line---out-schema-5)).  Two old words survive
  only as field names there: `nodes` counts steps, and `prev.source` says where the game of n − 1 came from.

## Programs, files and documents

* **the frame solver**: the crate `solver/frame/` and its binary `taxman`, a branch and bound over promotion sets
  within the frame model, which finds the games.  "The solver" for short.
* **the Java solver**: the original solver, whose algorithm the [wiki](https://github.com/bvchess/taxman/wiki) describes
  and the frame solver reimplements.  It is not in the tree.
* **the prover**: the crate `solver/noframe/`, a branch and bound over sets of moves on the whole board.  It decides
  playability by peel, uses only proven bounds, and shares no code with the frame solver.  It proves that no set of
  moves beats a recorded score ([its README](../solver/noframe/README.md)).
* **brute force** (`src/brute.rs`): the frame solver's exhaustive search, the test oracle for small games.
* **conjecture (★)**: on every component of the sources' incidence graph, and every truncation of it, descending
  greedy keeps a playable set of the largest size.  It is equivalent to THEORY.md's *upper-half conjecture*, that U\*
  is the heaviest playable set of sources.  **(♦)**: every source that descending greedy rejects is a GF(2)
  combination of sources it kept above it.  Statement (♦) implies (★) ([THEORY.md](THEORY.md), Theorem 7).
* **the heuristics** (`heuristics/`): strategies that play good games quickly but don't guarantee optimal results.
* **the probe** (`examples/perf.rs`), **the ladder** (`examples/ladder.rs`) and **`taxman bench`**: the performance
  tools of [solver/docs/design/performance.md](../solver/docs/design/performance.md).
* **the quick check** (`solver/tools/quick_check.py`): formatting, lints and the tests, within time limits.  Every
  change passes it.
* **the notebook**: the lab notebook, [solver/notebook/README.md](../solver/notebook/README.md): its rules, the
  register of everything tried (a file a subject, in `register/`), the index of rounds (`index.md`), and the entries,
  one folder each.  Older text calls it "EXPERIMENTS".

## Labels

Plans and theorems carry short labels.  A reference document gives the descriptive name first and the label once after
it.

* **A1 to A7**: the research avenues of [solver/docs/AGENDA.md, "Ranked
  avenues"](../solver/docs/AGENDA.md#3-ranked-avenues), such as A1, the multi-field bound, and A3, splitting the search
  by the number of promotions.
* **C1 to C7**: the conjectures of [solver/docs/AGENDA.md, "Conjectures"](../solver/docs/AGENDA.md#4-conjectures).
* **E-numbers** (E1, E6, E9(a), ...): the experiments of one plan, numbered within it.  The same number means
  different experiments in different plans; the entry or document that uses one defines it.  R1 to R3 likewise name
  the multi-field bound's refit procedures, and U4 a step of its plan; its entries define them.
* **Theorems, lemmas and statements** (Theorem 1, Theorem M, Lemma D, L1 to L4, M1 to M6, statements N and U): the
  results of [THEORY.md](THEORY.md).
