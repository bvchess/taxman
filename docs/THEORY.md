# The theory, with proofs

The theory proves the theorems that the project's programs rest on: the frame solver, which finds optimal games, and the
prover, which checks them on the whole board, and the claims of the heuristics.  Each result is stated in words, then
formally, then proved, and each says where it stands: proved, proved given a statement that is open, certified game by
game, measured, or open.  [statements.md](theory/statements.md) lists them all with their standing; the heuristics'
claims table ([heuristics/README.md](../heuristics/README.md)) cites them, and [evidence.md](theory/evidence.md) gives
the computations behind the statements that rest on computation.

## The files

Each file proves the results of one subject.  The theorems and lemmas keep their names, so a citation by name finds
its file in the last column.

| file | read it for | holds |
|---|---|---|
| [statements.md](theory/statements.md) | every statement with its standing and its file, and the notation | the table of statements; Definitions |
| [decision-procedure.md](theory/decision-procedure.md) | when a set of moves can be played, and the sets that cannot | Theorems 1 to 6, the extension lemma |
| [open-problem.md](theory/open-problem.md) | why exchange arguments fail, and what is open | Theorem 7, (★), (♦), the frontier argument, the independence lemma |
| [uniqueness-and-longest-games.md](theory/uniqueness-and-longest-games.md) | whether the optimal set is unique, and whether an optimal game plays the most moves | Theorems 8 and 9, the lemma of the prime |
| [bounds.md](theory/bounds.md) | the bounds on the whole game and on the promotions, over GF(2) and other fields | Theorems 10, 11, 13, 14 and 15 |
| [frame-model.md](theory/frame-model.md) | what the frame model proves, and what rests on statement U | Lemma Ω, Theorems L, B and M, M1 to M6, N, U |
| [extended-problem.md](theory/extended-problem.md) | the problem a certificate of U searches | Theorem M⁺, Lemma M5⁺ |
| [ledger.md](theory/ledger.md) | the bounds carried from one game to others, among them the ceiling opt(n) ≤ n + opt(n − 1) | Lemmas L1 to L4, D and F |
| [franklin-moniot.md](theory/franklin-moniot.md) | how these results relate to Franklín and Moniot's paper | Theorem 12, the bridge |
| [evidence.md](theory/evidence.md) | the computations behind the statements that rest on computation | |
