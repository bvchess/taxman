# The verifiers

Part of the [reference](REFERENCE.md).

The referee checks every game before it counts.  Two verifiers that share no code check results files, and both derive
each line's proof and assumptions again, so that a hand edit cannot change them.

* **The referee.**  Every `solve` replays each game through `game::replay_finished`, an independent referee that
  shares no code with the solver's number theory: the game must be legal, be over, open with the largest prime, and
  score what is recorded.  `solve` also checks the arithmetic and the ceilings `score(n-1) <= score(n) <= n +
  score(n-1)` before the game counts.  A game that fails after it was found goes to `<results>.rejected.jsonl`, and
  the run fails.
* **`taxman verify-results FILE...`** reads results files without changing them.  It replays every game, confirms its
  score against the built-in table, and checks the chain from n − 1, each line's arithmetic and provenance, and that
  the files agree.  It checks that a line's fields agree with each other as the solver writes them, so that a hand edit
  cannot drop an assumption or change a proof.  It counts the games by proof and by what they assume, and the searches
  that the near-repeat cut shortened (`stats.near_repeat_cut` above 0) by proof.  For a repeat it sets the search
  problems of both games up again and compares them.  For a `known` line it checks the citation against the sources
  ([the reference, "Citing a known source"](REFERENCE.md#citing-a-known-source)): the built-in table, the committed
  chain and the bounds store of the repository, read as they are, since their own verifiers check them.  Opening a
  results file for `--out` runs the same checks, with the sources that `--use-prev` draws on.
* **Both verifiers accept a file with holes**, such as a `--keep-going` run's or the sweep's `explore.jsonl`, whose
  games may have no line or only incomplete ones.  They check each line against what it rests on in the same file:
  * a ceiling (`setup.cap`), or a `cap` or `prime_formula` proof, needs the game of n − 1 completed in the file;
  * a repeat needs the game it repeats completed on an earlier line;
  * an exhausted search needs nothing of n − 1;
  * a line built on the best game of an unfinished search names an earlier incomplete line of n − 1 with that score and
    those moves, scores no less than it, and has no ceiling and no such proof;
  * an incomplete line that records its game (`best_so_far`, the moves) is checked to be legal, to score the
    `best_so_far`, and to be no better than the built-in table's score.

  The report names the games that only an unfinished search is recorded for.  A line whose game of n − 1 is not in the
  file at all is still refused.  `tests/results.rs` shows each case with both verifiers.
* **`python3 solver/tools/check_results.py [FILE...]`** is an independent verifier in Python: standard library only,
  and no code shared with the solver.  It derives every proof and assumption again, and also checks each score against
  the games of `results/optimal.jsonl.gz`, the Franklín–Moniot upper bound and the best-known heuristic chain in
  `heuristics/results`.  It reads a path ending in `.gz` through Python's `gzip` module.  With no arguments it checks
  `results/*.jsonl` and `results/*.jsonl.gz` but the store, `bounds.jsonl`, which is not a file of results.  It cannot
  rebuild the frame model, so it takes an `exhausted` line at its word.  Of a repeat it checks only what the lines show:
  the game repeated is earlier in the file and settles its search, and the recorded digests and promotions agree.  It
  checks a `known` line against the sources (`--committed FILE` and `--bounds FILE`, by default the committed chain
  and the store of `results/`), from the lines of the files alone.  It cannot play the store's promotion sets, so it
  does not check the digest of a game of n − 1 taken from the store.  It also asks that the method fits the nodes: a
  `start` line is completed with `nodes` 0 and a completed `search` line has more, which refuses a line of a build
  that called a `start` game a `search`.
* **Both verifiers read `stats.near_repeat_cut` as a count and not as a proof.**  A search that the near-repeat cut
  shortened is `exhausted` all the same, so the key changes no proof and no assumption.  Both refuse a value that is
  no count, and the key on a line that ran no search.  Neither refuses a line for lacking the key, as every line made
  before the cut does.

The report of `taxman verify-results results/optimal.jsonl.gz` gives the completed games and the range n = 1..last with
the number missing, the incomplete lines, the games by proof and by what they assume, the scores at a proven ceiling,
the searches shortened by the near-repeat cut by proof, the games in more than one file, and the time it took.

What the checks cannot catch is a game that is legal, finished and within the chain bounds but not optimal, when the
game of n − 1 is not optimal either.  That is what the proof kinds and the assumptions track, and why a chain from n =
1 matters.  [results/README.md](../../results/README.md#what-supports-the-scores) has the report of the committed file
as of a date, and says what supports each committed score: solves from scratch, a second build's chain, and what the
games assume.
