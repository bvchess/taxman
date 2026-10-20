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
it, maximal factors are enough: see the lifting argument of `numtheory` and [THEORY.md](../../../docs/THEORY.md)
(Theorem 1), which the [README](../../frame/README.md#how-the-solver-sees-the-game-the-frame-model) summarises.

`frames::BoardFrames` forms the [frames](../../../docs/GLOSSARY.md#the-frame-model) of a board, in playing order: the
frames [in front of](../../../docs/GLOSSARY.md#the-frame-model) a frame are played before it.  Setup simplifies each
frame in three ways.  It finds the frame's [mandatory moves](../../../docs/GLOSSARY.md#the-frame-model) (`unzip`), drops
its [impossible sources](../../../docs/GLOSSARY.md#the-frame-model) (`removeImpossible`), and counts its
[freedoms](../../../docs/GLOSSARY.md#the-frame-model); a [closed](../../../docs/GLOSSARY.md#the-frame-model) frame has
none.

`frames::build_frames` numbers the frames that have freedoms by [layer](../../../docs/GLOSSARY.md#the-frame-model), from
the front, and merges the frames of one [frame number](../../../docs/GLOSSARY.md#the-frame-model), so that a promotion
always has one frame to go to; the README's example shows where that happens.  The glossary's [frame
number](../../../docs/GLOSSARY.md#the-frame-model) compares the numbering with the Java's.

Each frame plays [Taxman Mini](../../../docs/GLOSSARY.md#the-frame-model), and is
[winnable](../../../docs/GLOSSARY.md#the-frame-model) when some order takes every move.  The module `model` tests that
with counters instead of searching orders, by [peel](../../../docs/GLOSSARY.md#the-game).

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
[docs/theory/frame-model.md](../../../docs/theory/frame-model.md) proves what that is:

* The sources that setup keeps are [U\*](../../../docs/GLOSSARY.md#the-frame-model), chosen greedily from the largest on
  each frame, and in_the_bag = ΣU\* (Theorem M, M1).
* **Completeness and soundness** (Theorem M): a set U\* ∪ L, with L a set of numbers up to n/2, is playable exactly when
  L is a promotion set of Π_n, and then it scores in_the_bag + ΣL.  So no step of setup loses a game that plays U\*, and
  none admits a set that is not a game.  The steps are `unzip`'s mandatory moves, freedoms = factors − sources, closed
  frames taking no part, a promotion paying one frame forward, the numbering, the candidate filter and the root's
  reduction.  What the proof needs of the numbering is that it is [clean](../../../docs/GLOSSARY.md#the-frame-model),
  which the numbering by layer is by construction.
* Every optimal game plays the members of U\* with one or two prime factors (Theorem B).
* So in_the_bag + P\*(Π_n) = opt(n) exactly when [statement U](../../../docs/GLOSSARY.md#the-frame-model) holds at n:
  some optimal game plays the members of U\* with three or more prime factors.

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
barred bound certifies is played by every set of moves that beats the recorded score and obeys Theorem B, and the
members it does not certify are *open*.  It certifies most members in each game, within the prover's proofs and past
them, but closes no game alone
([solver/noframe/docs/proofs.md](../../noframe/docs/proofs.md#the-upper-part-game-by-game) has the
method, and [the entry](../../notebook/the-upper-part-game-by-game-and-two-cloud-runs-prepared-2026-10/README.md) the
shares).

Second, a search settles the open members.  It is the search of the [extended
problem](../../../docs/GLOSSARY.md#the-frame-model) Π⁺(O), with the set O of open members optional (THEORY.md, Theorem
M⁺; `frames::build_extended`, research code).  The extended problem is exact like Π_n, so a search of it that finds
nothing better, with the other members certified, proves opt(n) on the whole board.  The search takes the open members
as one-sided candidates ([search.md](search.md#extended-mode-one-sided-candidates); `Board::prepare_extended_with`).

Details of the extended problem: it keeps the claims of `unzip` that Lemma M5⁺ (THEORY.md) allows.  With nothing open
that is all of them, so Π⁺(∅) is Π_n itself.  Without the claims, Π⁺(∅) is the frame model with them put back
(`examples/extended.rs --no-claims`).  A hunt for a counterexample on the boards without each open member, the example
`hunt`, was prepared and never run.  It is removed from master, and [its
entry](../../notebook/the-upper-part-game-by-game-and-two-cloud-runs-prepared-2026-10/README.md) says where it builds
("the program removed from master").

What each kind of proof needs of the frame model, beyond the code's correctness and the games before it:

| proof | needs |
|---|---|
| `prime_formula` | nothing of the model (proved at `accelerate::prime_shortcut`) |
| `cap` | nothing of the model: the game is replayed, and reaches the ceiling n + score(n − 1) ≥ opt(n) given that score(n − 1) = opt(n − 1) |
| `exhausted`, `trivial` | completeness at n, so U at n |
| `same_search` of an exhausted game m | U at n, and Π_n = Π_m (checked) |
| `same_search` of a `cap` game m | U at n, Π_n = Π_m (checked), and soundness at m (Theorem M), which makes m's ceiling P\*(Π_m) |
| `known` | nothing of the model at n: what the cited game or bound assumes (for the bounds store, U at the games its bounds rest on) |

The tests tie the theorems to the code:

* `tests/model_exact.rs` checks Theorem M both ways, on every set of every board to 100 and on random sets to 8191,
  against a playability oracle that shares no code with `frames` or `model`.
* `tests/model_extended.rs` checks Theorem M⁺ the same way, with sources open.
* `tests/bottom_layers.rs` checks Theorem B on every recorded and enumerated optimal set.
* Two exhaustive searches agree with the frame model's games on small boards: `brute`, the solver's own, to 40 in the
  test suite, and `heuristics/evaluation/certify.py`, to 62.  The example `sacrifice`, removed from master, once ran
  `brute` to 52 ([the entry](../../notebook/the-frame-model-by-layer-and-the-extended-problem-2026-10/README.md) has the
  run).

The sacrifice bound (`sacrifice`) is an upper bound on opt(n) by a different route: games on punctured boards, boards
with holes, where Theorem M holds too.  But U and Theorem B are statements about the board 1..n, so the bound rests on
the upper part of each punctured board.  It proves game 1133 and disagrees with nothing.

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
space, and it reproduces the Java's values exactly; the Java's golden data in `testdata/golden/` is the check.  Every
final score equals optimal.json's.

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
