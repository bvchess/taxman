# The unsound mode

The unsound mode plays each game of a range as well as a level of effort finds, and gives no proof that its score is the
best.  It is for a user who wants scores fast and accepts that none is proven.  A proof of a hard game takes hours,
and nearly all of it goes to proving a set that the search found early; this mode keeps the finding and drops the proof.
In the files of this folder, `taxman` is `solver/frame/target/release/taxman`, run from the top of the repository ([the
frame solver's README](../frame/README.md) says how to build it), and the words of the search are in the
[glossary](../../docs/GLOSSARY.md#the-search).

## What it is and is not

Each line of a run is a real game.  Its moves are legal and are replayed before they count, so its score is a lower
bound on opt(n), the best score of game n.  The score is opt(n) if the search found the best game, and nothing in the
run says whether it did.  A higher level of effort buys a better chance, with more time, and its seeded play never
scores less than a lower one's, since it plays the rounds of the lower one first.  A score above the built-in table's,
which is proven, or above the game's [bound](unsound/shortfalls.md#the-bound-and-the-gap) is a hard error.

An unsound line never mixes with proven ones.  Its key `"unsound":true` makes `solve` without `--unsound`,
`verify-results`, `solver/tools/check_results.py` and the built-in table refuse any line that has it, and `--unsound`
refuses a file that holds a results line.  So a file of unsound lines is a file of its own, and nothing takes it for a
proof.

## The files

| topic | file | read it to |
|---|---|---|
| running the mode | [run.md](unsound/run.md) | run it: the command, what it prints, and every flag |
| playing a game | [play.md](unsound/play.md) | choose the level of effort and the options that shape the rounds: how a game is played, what the levels gave, the run's ceiling |
| short games | [shortfalls.md](unsound/shortfalls.md) | see how a game that falls short is pulled up or played again: the repair, games played twice, and the bound and the gap that stop or push a game |
| the best-known file | [best-known.md](unsound/best-known.md) | keep the best-known line of each game across runs: what the file is, how a run starts from it and adds to it, and how to fold an old file in |
| the line | [line.md](unsound/line.md) | read a line of the file: an example, and every key |
