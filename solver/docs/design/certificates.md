# Certificates of U

Part of the [design](../DESIGN.md).

A *certificate* proves statement U at one game: that the recorded score of a game whose proof uses the frame model
(`exhausted`, `trivial`, `same_search`) is opt(n) on the whole board ([frame-model.md](frame-model.md)).
`taxman certify` makes certificates game by game, by the corollary of Theorem M⁺ (docs/theory/extended-problem.md),
and `taxman verify-certificates` checks them.  [results/README.md](../../../results/README.md#certificates) says which
games have one.

*What `certify` does* (`taxman certify (A-B | --list FILE) --results FILE... --upper FILE... --out FILE`;
`src/certify.rs`).  It certifies the games whose lines in the prover's barred parity bounds
(`solver/noframe/data/upper_*.jsonl`) say they need a certificate.  For such a game it takes the line's open members O,
and sets up the extended problem Π⁺(O) in extended mode ([search.md, "Extended mode: one-sided
candidates"](search.md#extended-mode-one-sided-candidates)), with the other members of U\*_{≥3} fixed.  It searches that
problem from the recorded game, as the promotion set (O, its moves up to n/2), with the ceiling n + score(n − 1) −
in_the_bag⁺.  Before the search it checks that the game replays to its score, plays U\* above n/2, and is a promotion
set of Π⁺(O) worth the score (Theorem M⁺ says that a playable one is).  The search ends with one of two verdicts:

* *certified*: the search ends with nothing better, so opt(n) is the score, given that the members not open were
  certified against it.  A line of the bounds whose score is above the recorded one certifies nothing against it, and
  is refused.
* *beaten*: the search found a promotion set worth more.  Its set F ∪ K ∪ L is played in the order of the peel of
  `playable` (the whole-board oracle, which shares nothing with the frame model), finished with the largest legal move
  left until there is none, and refereed by `game::replay_finished`.  It is a counterexample to the score, and to U if
  the score rests on the frame model.  It goes to `<out>.counterexample.jsonl` as well, and the run exits with the
  status 2.

*The formulation.*  Π⁺(O) has two formulations (`Board::extended`; THEORY.md, Lemma M5⁺): with the claims of `unzip`
that the lemma allows to keep (the default), and without any (`certify --no-claims`, the formulation of every
certificate made before the lemma).  The two have the same promotion sets, and so the same heaviest promotion set (the
lemma's corollary), so a certificate made in one bounds the heaviest promotion set of the problem in the other.  A line
records the formulation it was made in (`claims`, true or false); a line with no such field reads as false.

*The same problem.*  Consecutive games often pose the same Π⁺(O) (the same `search::Problem`, which has no n in it:
"Same search"), and then an earlier game's certificate settles the later one with no search.  If game m is certified,
by a search or by this rule, and Π⁺(n) equals Π⁺(m), then the heaviest promotion set of Π⁺(n) is worth best(m) =
score(m) − in_the_bag⁺(m), which m's certificate proved, and a recorded game of n worth as much has nothing above it.

* *Where it looks.*  After the checks of n's recorded game and before its search (a checkpoint of n, if there is one,
  changes nothing), `certify` looks at the certified games of the `SAME_SEARCH_WINDOW` = 8 games before n, those of the
  file `--out` and those of the run, latest first, and never further.
* *What it compares.*  It compares the problems in the run's formulation, whatever formulation m's certificate was made
  in.  It passes over a game of the run's formulation whose line records another digest than Π⁺(n)'s, and a game of
  the other formulation (whose digest says nothing of Π⁺(n)'s) unless its frames and candidates are Π⁺(n)'s, which the
  claims do not change.  For a game that is not passed over, m's problem in the run's formulation is the one the run
  set up when it certified m (kept for the last eight games, and only if m's line is of the run's formulation).  For a
  line of the file it is set up again from m's game and the open members the line records (`certify::extended_problem`),
  in the line's own formulation, which must reproduce the line (its formulation, digest, frames, candidates, rejected
  sources and in_the_bag⁺), or the line is dropped; then, for a line of the other formulation, it is set up again in the
  run's.  `certify` compares it with Π⁺(n) whole, so that the answer never rests on two digests alone, and the first
  game with an equal problem is m.
* *What it concludes*, with worth(n) = score(n) − in_the_bag⁺(n):
  * worth(n) = best(m): n is *certified*, with `nodes` 0 and `"same_problem": {"n": m, "digest": ..., "best": ...,
    "worth": ...}` in its line;
  * worth(n) < best(m): the recorded game is not optimal, so the rule is not used and n is searched (the search finds
    the better set and says *beaten*);
  * worth(n) > best(m): impossible with genuine data (the recorded game of n is a promotion set of a problem that m's
    certificate says nothing beats), so the run fails, naming n, m and both values, and certifies nothing.

The rule is sound because equal problems have equal heaviest promotion sets: the form holds the frames, the
candidates, `max_total` and the graph's lists, and nothing of n.  The heaviest promotion set of a problem does not
depend on the formulation it was certified in, so nothing beats n's recorded game, and n's own barred bounds then give
opt(n) = score(n) by the corollary, exactly as after a search.  The refusal of a line of the bounds that was found
against a higher score applies to a game the rule settles too.

*The line.*  `--out` gets one line a game: the game, the score, the open members, how many sources greedy rejected
beside them, the frames and candidates of Π⁺(O), the nodes and seconds of the search, the threads and set width, the
formulation (`claims`), the verdict, the digest of Π⁺(O), the game it rests on if the rule settled it
(`same_problem`), and the build.  A search stopped by a limit or a signal writes an `incomplete` line and keeps its
checkpoint (the flags are `solve`'s); a rerun skips the certified games, stops again at a beaten one, and resumes the
others.  A problem whose
GF(2) bound needs more block words than the engines of the game's set width have is searched at the widest set width
(128 words, five words of engine); `certify::certificate_width` is the one place that decides the width, and the line
records it.

*A list of games.*  `certify` takes the games of a list file (`--list FILE`) in place of a range `A-B`, exactly one of
the two.  The file has one n a line, in the order to certify them; it skips blank lines and those whose first non-blank
character is `#`, and has no game twice.  Any other line is a usage error that names the file and the line.  The loop
of the range takes the games in the file's order, with the same rules: the game's line of the barred bounds must exist,
a game that needs no certificate is counted (`not needing one`) and passed over, a `certified` verdict in `--out` skips
the game (a `beaten` one stops the run again), and the rule of the same problem looks at the certified games of the file
and of the run, whatever order they
were made in.  The tally line reads `list FILE (K boards): ...` where a range gives `A..=B: ...`.

*The campaign's lists.*  `solver/tools/certify_plan.py` (standard library only) makes the lists of the campaign on
2176-3440, and its estimate, from the results, the barred bounds, the extended problems of
`examples/extended.rs problems` (which games the rule will settle) and the root gaps.  A game that the rule settles goes
into the list of the game it rests on, after it, since the rule reads only its own `--out`
([solver/tools/certify/](../../tools/certify/README.md)).  `solver/tools/certify_campaign.py` cuts the campaign's phases
from that work list into `solver/tools/certify/lists/stage4/` (a pilot of seven dear games with a stop rule, the dear
games, four balanced lists, and the repeats the rule cannot settle), `solver/tools/certify_pilot.py` rates the pilot,
and `solver/tools/cloud/certify.sh` runs them ([runbook/certify.md](../runbook/certify.md)).

*Verifying the certificates.*  `taxman verify-certificates CERTS... --results FILE... --upper FILE... [--exact FILE...]
[--threads K]` (`src/certify/verify.rs`) reads the files that `certify --out` wrote, without changing them, and checks
every line.  It notes and ignores a last line without its newline, as the file cuts it when it is opened.  A complete
line that is no certificate, or whose `build` lacks the commit (40 hexadecimal digits) or the `dirty` flag, is an
error.  It counts and skips an `incomplete` line, and checks a line with a final verdict on these points:

1. the barred bounds have a line for the game, which says it needs a certificate, and whose open members and score
   are the line's;
2. the results have a game of n with the line's score, the game passes the checks that `certify` makes of it (it
   replays to its score, plays U\* above n/2 and is a promotion set of Π⁺(O) worth the score: `check_recorded_game`,
   shared by the two), and the line's `width` is the one `certificate_width` gives; both are set up in the line's own
   formulation (`claims`);
3. Π⁺(O) set up again reproduces the line: its formulation, digest, frames, candidates, rejected sources and
   `in_the_bag⁺`;
4. a `certified` line has no `beaten_by`, and the file of counterexamples beside the certificates
   (`<certificates>.counterexample.jsonl`) has no line for its game;
5. a `certified` line with `same_problem` {m, digest, best, worth} rests on a game m among the 8 games before it, whose
   last final line before it in the same file is `certified`.  The rule of the same problem (`apply_rule`, the function
   that `certify` applies, so that the rule is written once) is applied again to the two problems, which must be equal
   whole in the formulation of the line that rests on m (m is set up again in it if m's line is of the other), and must
   give exactly the `same_problem` that the line records (the digest, `best` = score(m) − `in_the_bag⁺`(m), and `worth`
   = the line's score − `in_the_bag⁺`, equal);
6. a `beaten` line has a finished legal game (`game::replay_finished`) that scores what the line says, which is more
   than the recorded score, and the counterexample file has an identical line;
7. a game with final lines in more than one place (one file or several) has only `certified` ones: a `beaten` line
   beside another is a contradiction of verdicts.

The first line to fail ends the command with the status 1, naming the file, the line and the game.  The dear checks
(the problems set up again) run on K threads (default: all CPUs), and the first line to fail is the same for any K.
What the command cannot check is that a search explored its whole tree: it takes a `certified` line made by a search on
trust, as an exhausted proof is, and the summary says so.  The summary has, for each file, the lines, the final and
`incomplete` ones, the `certified` ones by search and by the same problem, and the `beaten` ones; then the games
certified; the counterexamples (the game, the recorded score, the score of the game that beats it); the builds that
wrote the lines (commit, `dirty`, lines); and the proven end of the chain.

*How far the proofs reach.*  A game n, from 1 to the largest game of the results, is *proven* when n = 1, or the prover
proved it, or it has a certificate that passed, or its score reaches a proven ceiling and the game of n − 1 is proven:

* the prover's proofs are the lines of `--exact` (by default every `solver/noframe/data/exact_*.jsonl`, by name; a
  later file's line for a game replaces an earlier file's, and the lines with `fix_upper` set are measurements, and
  skipped).  Their score must be the results' (a line with another score is an error naming the game and both
  scores).  A line proves its game by the rule of `exact_proofs` in `solver/noframe/tools/summarize.py`: its
  Franklín–Moniot bound or its parity bound is the score, or its search is `proven` (a search that used the segment
  cuts counts only if every smaller game is proven), or its relation is `chain` or `prime` and the game of n − 1 is
  proven;
* a ceiling is `results::CeilingProof`: the score n + score(n − 1), or the prime formula.

The command prints `proven frontier: every game to N is proven; the first that is not is M (reason)`, where the reason
is that the results have no game of M, or that it needs a certificate (the barred bounds say so) and the files given
have none.  Then it says how many games past M are proven all the same: those that no gap stops (a bound of the
prover, a certificate).  `verify::frontier` is that function of plain data (the results' scores, the prover's lines,
the games certified).  With the committed prover files and no certificate, the end is that of the prover's proofs,
whose first game past them needs a certificate ([results/README.md](../../../results/README.md#what-supports-the-scores)
says how far they reach).
