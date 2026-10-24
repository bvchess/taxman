# The game and the frame model

Part of the [design](../DESIGN.md).

The solver does not search the game move by move.  It splits the numbers into independent groups, the
[frames](../../../docs/GLOSSARY.md#the-frame-model), settles what it can at setup, and then searches only over which
spare numbers to [promote](../../../docs/GLOSSARY.md#the-frame-model).  The [glossary](../../../docs/GLOSSARY.md)
defines each term and the [README](../../frame/README.md#how-the-solver-sees-the-game-the-frame-model) walks through the
board 1 to 21; this section names the code that does each step, and says what the solver's answer means.

## The game

The rules are those of the glossary's [game](../../../docs/GLOSSARY.md#the-game).  In the code, `game::replay` applies
them to a list of moves.  Its companion `game::replay_finished` also requires the game to be over and to open with the
largest prime, as every optimal game does.  Together they are the [referee](../../../docs/GLOSSARY.md#results-and-trust)
of a game, and they share no code with the solver.

## Sources, factors, frames

On a board, the [sources](../../../docs/GLOSSARY.md#the-frame-model) are the numbers above n/2 and the
[factors](../../../docs/GLOSSARY.md#the-frame-model) the rest.  A move is paid for by one of its [maximal
factors](../../../docs/GLOSSARY.md#the-game), and a factor pays for one move.  Although a move takes every divisor of
it, maximal factors are enough: see the lifting argument of `numtheory` and [the lifting lemma
(P1)](../../../docs/theory/playability.md#the-lifting-lemma), which the
[README](../../frame/README.md#how-the-solver-sees-the-game-the-frame-model) summarizes.

`frames::BoardFrames` forms the [frames](../../../docs/GLOSSARY.md#the-frame-model) of a board, in playing order: the
frames [in front of](../../../docs/GLOSSARY.md#the-frame-model) a frame are played before it.  Setup simplifies each
frame in three ways.  It finds the frame's [mandatory moves](../../../docs/GLOSSARY.md#the-frame-model) (`unzip`), drops
its [impossible sources](../../../docs/GLOSSARY.md#the-frame-model) (`removeImpossible`), and counts its
[freedoms](../../../docs/GLOSSARY.md#the-frame-model); a [closed](../../../docs/GLOSSARY.md#the-frame-model) frame has
none.

`frames::build_frames` numbers the frames that have freedoms by [layer](../../../docs/GLOSSARY.md#the-frame-model), from
the front, and merges the frames of one [frame number](../../../docs/GLOSSARY.md#the-frame-model), so that a promotion
always has one frame to go to; the README's example shows where that happens.  [Setup in the code](#setup-in-the-code)
compares the numbering with the Java's.

Each frame plays [Taxman Mini](../../../docs/GLOSSARY.md#the-frame-model), and is
[winnable](../../../docs/GLOSSARY.md#the-frame-model) when some order takes every move.  The module `model` tests that
with counters instead of searching orders, by [peel](../../../docs/GLOSSARY.md#the-game).

## Setup in the code

The theory defines setup's steps as operations on the board ([the frame
model](../../../docs/theory/frame-model.md#the-frame-model)).  This section names the code of each step, and says
where the code asks more than the theory's rules.

1. `BoardFrames::new` finds the sources and the lists of who can pay for whom (`factors_of`, `composites_of`), forms the
   frames by a union-find over the edges from each source to its factors, and orders them so that a frame comes after
   the frames in front of it.  Then `finish_setup` simplifies them.
2. `finish_setup` runs `remove_impossible`, descending greedy over a frame's remaining sources against its remaining
   factors, on every frame.  Then, round after round over the frames that changed and their neighbors, `finish_frame`
   runs `unzip`, which makes the claims, and `remove_impossible` again, until nothing changes.  The later passes of
   `remove_impossible` keep every source ([the kept-sources lemma
   (X1)](../../../docs/theory/exactness.md#the-exactness-theorem)), so a frame's `swept` flag skips them.  `unzip` makes
   back claims (`ClaimKind::Back`) in a loop over the factors, ascending, and front claims (`ClaimKind::Front`) in a
   loop over the sources, descending, which starts again after each claim.
3. `frames::build_frames` numbers the frames with freedoms by layer (`numbered_frames`, `layer_of`).  Number by number,
   it collects the candidates of each frame (`promotion_candidates_of`) and then merges the frames of that number
   (`merge`).  After the last merge it calls `finish_setup` again, since a merged frame starts with nothing found.
   `SearchFrames` holds, for each number, the merged frame's remaining sources and factors, its claims, and the
   candidates.
4. `model::FrameGraph::build` reduces each frame (`Frame::reduce`) and records each number's home frame.

The code's tests against the theory's:

* **Test (a).**  `promotion_candidates_of` tests a frame's remaining sources against its remaining factors less x, after
  the claims of step 2.  [The claims lemma (X5)](../../../docs/theory/exactness.md#the-exactness-theorem), part 3, shows
  that this picks the candidates of the theory, which tests the whole frame.
* **Test (b).**  `promotion_candidates_of` asks whether some frame that holds a maximal factor of x can take x on
  (`can_add_source`).  It asks once the frames of the number below are merged, so it asks the theory's question whether
  frame h − 1 can take x on; a closed frame can take nothing on.
* **Back claims.**  `unzip` asks that a single remaining source want f, that f cannot go forward
  (`factor_can_go_forward`: no frame that holds a maximal factor of f can take f on), and that f has no outside move
  (`has_outside_move_using`: no composite of f in another frame with freedoms can be taken on by f's frame).  Before the
  merges the frames asked are frames; after them, merged frames, which can take a number on iff one of their frames can.
  Every factor of a frame with freedoms that has f as a maximal factor is such a composite, so every back claim meets
  the theory's rule.
* **Front claims.**  `unzip` asks too that f have no outside move.  The theory's rule does not need it: a front claim is
  a front move whatever is promoted into its frame (X5).

**Setup builds Π_n.**  The theory's root problem Π_n makes the claims of each frame h in one canonical order ([the frame
model](../../../docs/theory/frame-model.md#the-frame-model)), and setup makes its claims in rounds, before and after
merging, counting every frame's freedoms as they are.  The two meet at the root: setup's U\*, candidates and root frames
are those of Π_n.  The front claims that setup's outside test blocks are dropped by its final reduction, which leaves
the root frames as Π_n's.  `src/root_problem.rs` builds Π_n from the theory's definitions, apart from the frames code,
and `tests/canonical_root.rs` compares setup with it on every board to 1000, check I of the census on every board to
8191.

**The Java's numbering.**  The Java numbered the frames breadth first from the front: 0 for those with no frame with
freedoms in front of them, h + 1 for those directly behind a frame numbered h and not yet numbered.  That is the
numbering by layer when it is clean and its frames numbered 0 lie in one layer.  The census of every board to 8191, the
largest the solver supports (`examples/structure.rs`; the data in
`solver/notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/`), found the two the same on every one, with
frame h in layer 3 + h on 8,100 boards and in layer 2 + h on the 80 where the frames of layer 2 have freedoms.  The
tests of `src/frames/` compare the two numberings on every board to 400 and on a sample to 8191.  On boards with holes
they can differ: without 39, the board of 56 gets a frame and a frame with freedoms directly in front of it numbered
alike by the breadth-first rule, and without 4 the board of 32 has a layer with only closed frames between two with
freedoms.

**The census** (`examples/structure.rs`) checks the theory's facts against what setup builds, on every board to 8191:
check A [the frame lemma (F1)](../../../docs/theory/frame-model.md#the-frame-lemma), C [clean numbering
(F6)](../../../docs/theory/frame-model.md#clean-numbering), D that no candidate lists a back claim's factor (X5), E that
no candidate is a factor of frame 0, F that every candidate has a home of at least 1 (the reduction-step lemma (X6)), G
that the game with no promotions plays the U\* of `src/root_problem.rs` (X1), H that the extended problem with nothing
open and the claims poses the frame model's problem ([the formulation corollary,
E5](../../../docs/theory/extended-problem.md#the-kept-claims-lemma)), and I that setup builds Π_n.  The theory proves
all of these, so the census checks the code.  `tests/model_exact.rs` finds the boards where test (b) alone keeps an
unplayable set out of the promotion sets: 27 boards to 2000, all in 98–120 and 182–186, the first the theory's example
at 98.

## Promotions and the score

Once setup has settled the frames, what a game can still choose is which spare factors to
[promote](../../../docs/GLOSSARY.md#the-frame-model).  The score of a game of the model is `in_the_bag`, what the game
with no promotions scores, plus the sum of its promotions.  So the solver looks for the [promotion
set](../../../docs/GLOSSARY.md#the-frame-model) with the largest sum.  The moves of the game follow from the promotions
(`frames::BoardFrames`, with the promotions treated as sources).  The numbers that could be promoted are the
[candidates](../../../docs/GLOSSARY.md#the-frame-model), and the glossary has the rules of a promotion: one frame
forward, a freedom spent, the frames reduced.

## What "optimal" means

The solver's answer for n rests on up to three premises: the frame model is exact at n, the search's pruning is exact,
and the game of n − 1 is optimal.

**Within the frame model.**  The search finds [P\*](../../../docs/GLOSSARY.md#the-frame-model)(Π_n), the largest sum of
a promotion set of the root problem Π_n ([same-search.md](same-search.md), defines the [search
problem](../../../docs/GLOSSARY.md#the-frame-model) Π exactly).  The answer is in_the_bag + P\*(Π_n).
[docs/theory/exactness.md](../../../docs/theory/exactness.md) proves what that is:

* The sources that setup keeps are [U\*](../../../docs/GLOSSARY.md#the-frame-model), chosen greedily from the largest on
  each frame, and in_the_bag = ΣU\* ([the kept-sources lemma,
  X1](../../../docs/theory/exactness.md#the-exactness-theorem)).
* **Completeness and soundness** ([the exactness theorem
  (X7)](../../../docs/theory/exactness.md#the-exactness-theorem)): a set U\* ∪ L, with L a set of numbers up to n/2, is
  playable exactly when L is a promotion set of Π_n, and then it scores in_the_bag + ΣL.  So no step of setup loses a
  game that plays U\*, and none admits a set that is not a game.  The steps are `unzip`'s mandatory moves, freedoms =
  factors − sources, closed frames taking no part, a promotion paying one frame forward, the numbering, the candidate
  filter and the root's reduction.  What the proof needs of the numbering is that it is
  [clean](../../../docs/GLOSSARY.md#the-frame-model), which the numbering by layer is by construction.
* Every optimal game plays the members of U\* with one or two prime factors ([the bottom-layers theorem
  (F4)](../../../docs/theory/frame-model.md#the-bottom-layers-theorem)).
* So in_the_bag + P\*(Π_n) = opt(n) exactly when [statement U](../../../docs/GLOSSARY.md#the-frame-model) holds at n:
  some optimal game plays the members of U\* with three or more prime factors ([the corollary of U
  (X8)](../../../docs/theory/exactness.md#statement-u)).

The statement U is open in general.  It holds at every game that the
[prover](../../../docs/GLOSSARY.md#programs-files-and-documents) has proven, since the prover proves opt(n) on the whole
board; [results/README.md](../../../results/README.md#what-supports-the-scores) says how far its proofs of the committed
chain reach.  The prover ([its README](../../noframe/README.md)) is a separate program that uses nothing of frames;
[the section on its proofs](../../noframe/docs/proofs.md) lists the facts that they use.

Past the prover's proofs, the committed games are proven within the frame model, so each is opt(n) wherever U holds at
n.  Proving U at such an n takes two steps, which the command `taxman certify` combines game by game
([certificates.md](certificates.md)); [results/README.md](../../../results/README.md#certificates) says which games have
certificates.

First, the prover's [barred bounds](../../../docs/GLOSSARY.md#bounds) settle most members of U\*: each member that a
barred bound certifies is played by every set of moves that beats the recorded score and obeys the bottom-layers
theorem, and the members it does not certify are *open*.  It certifies most members in each game, within the prover's
proofs and past them, but closes no game alone
([solver/noframe/docs/proofs.md](../../noframe/docs/proofs.md#the-upper-part-game-by-game) has the method, and [the
entry](../../notebook/the-upper-part-game-by-game-and-two-cloud-runs-prepared-2026-10/README.md) the shares).

Second, a search settles the open members.  It is the search of the [extended
problem](../../../docs/GLOSSARY.md#the-frame-model) Π⁺(O), with the set O of open members optional ([the extended
exactness theorem (E1)](../../../docs/theory/extended-problem.md#the-extended-exactness-theorem);
`frames::build_extended`, research code).  The extended problem is exact like Π_n, so a search of it that finds nothing
better, with the other members certified, proves opt(n) on the whole board.  The search takes the open members as
one-sided candidates ([extended-mode.md](extended-mode.md); `Board::prepare_extended_with`).

Details of the extended problem: it keeps the claims of `unzip` that [the kept-claims lemma
(E4)](../../../docs/theory/extended-problem.md#the-kept-claims-lemma) allows.  With nothing open that is all of them, so
Π⁺(∅) is Π_n itself.  Without the claims, Π⁺(∅) is the frame model with them put back (`examples/extended.rs
--no-claims`).  A hunt for a counterexample on the boards without each open member, the example `hunt`, was prepared and
never run.  It is removed from master, and [its
entry](../../notebook/the-upper-part-game-by-game-and-two-cloud-runs-prepared-2026-10/README.md) says where it builds
("the program removed from master").

What each kind of proof needs of the frame model, beyond the code's correctness and the games before it:

| proof | needs |
|---|---|
| `prime_formula` | nothing of the model (proved at `accelerate::prime_shortcut`) |
| `cap` | nothing of the model: the game is replayed, and reaches the ceiling n + score(n − 1) ≥ opt(n) given that score(n − 1) = opt(n − 1) |
| `exhausted`, `trivial` | completeness at n, so U at n |
| `same_search` of an exhausted game m | U at n, and Π_n = Π_m (checked) |
| `same_search` of a `cap` game m | U at n, Π_n = Π_m (checked), and soundness at m (X7), which makes m's ceiling P\*(Π_m) |
| `known` | nothing of the model at n: what the cited game or bound assumes (for the bounds store, U at the games its bounds rest on) |

The tests tie the theorems to the code:

* `tests/model_exact.rs` checks the exactness theorem (X7) both ways, on every set of every board to 100 and on random
  sets to 8191, against a playability oracle that shares no code with `frames` or `model`.
* `tests/model_extended.rs` checks the extended exactness theorem (E1) the same way, with sources open.
* `tests/bottom_layers.rs` checks the bottom-layers theorem (F4) on every recorded and enumerated optimal set.
* Two exhaustive searches agree with the frame model's games on small boards: `brute`, the solver's own, to 40 in the
  test suite, and `heuristics/evaluation/certify.py`, to 62.  The example `sacrifice`, removed from master, once ran
  `brute` to 52 ([the entry](../../notebook/the-frame-model-by-layer-and-the-extended-problem-2026-10/README.md) has the
  run).

The sacrifice bound (`sacrifice`) is an upper bound on opt(n) by a different route: games on punctured boards, boards
with holes, where the exactness theorem holds too ([the holes theorem
(X9)](../../../docs/theory/exactness.md#boards-with-holes)).  But U and F4 are statements about the board 1..n, so the
bound rests on the upper part of each punctured board.  It proves game 1133 and disagrees with nothing.

**Given the pruning is exact.**  Every [tactic](../../../docs/GLOSSARY.md#the-search) is proven or sound;
[tactics.md](tactics.md) grades each.  The Java used two shortcuts that rest on conjectures, `reuse_prev` and
`revise_freedoms`.  The frame solver had them as tactics for a while, measured that they no longer paid, and removed
them ([which tactics earn their keep](../../notebook/search/which-tactics-earn-their-keep-2026-10/README.md)).

**Given the previous game is optimal.**  The game of n is built from the game of n − 1, using the
[ceiling](../../../docs/GLOSSARY.md#the-game) and the [prime formula](../../../docs/GLOSSARY.md#the-game).  So a chain
of games from n = 1 is proven game by game ([trust.md](trust.md)).

Optimal games need not be unique, though, and that matters to any step that reuses the game of n − 1.  From n = 425 on,
some games have two or three [optimal sets](../../../docs/GLOSSARY.md#the-game).  In all of them the members above n/2
are U\*; they differ below n/2.  For example, the recorded game of 685 plus the move 686 is unplayable, although
opt(686) = 686 + opt(685): the equality holds through another optimal set of 685.  So a chain step is decided by the
complete list of the optimal sets of n − 1, not by one of them
([solver/noframe/docs/uniqueness.md](../../noframe/docs/uniqueness.md)).

## The Java solver and the frame solver

The frame solver uses the Java solver's strategy, which the [wiki](https://github.com/bvchess/taxman/wiki) describes:
split the game into frames of Taxman Mini, and search for the best promotion set.

What must match the Java is the *results*.  The setup (frames, candidates, the greedy incumbent) defines the search
space, and it reproduces the Java's values exactly, except where the Java's claims are not those of Π_n; the Java's
golden data in `testdata/golden/` is the check.  On eight small boards (8, 12, 13, 16, 17 and 25 to 27) the Java counted
a frame it had not yet simplified as having freedoms, which blocked a back claim and left a frame another factor;
`tests/golden_frames.rs` lists these departures with the reason, and the games are the same.  Every final score equals
optimal.json's.

The search is the frame solver's own.  It started as the Java's loop: its bound, its depth-2 preclusion table, the
promotability checks of its candidate lists, and branching on the largest candidate.  Each of those was replaced by
something measured to be faster, then removed.  The notebook has the rounds, and [which tactics earn their
keep](../../notebook/search/which-tactics-earn-their-keep-2026-10/README.md) has the last measurements of what was
removed.

The wiki's words map onto this document's:

| the wiki | here |
|---|---|
| *selection* | a move |
| *front* and *back* | the same |
| the upper bound N + opt(N − 1) | the ceiling |

The port found one defect in the Java's bound: it can be too low.  Its runner-up replaces the
failing number with the next candidate below the smallest bid member, but a skipped candidate from the failing number's
frame can be larger.  At the root of n = 114, the Java's bound is 358 against the best completion's 360 ([the
story](../../notebook/search/the-port-and-its-first-improvements/README.md)).
