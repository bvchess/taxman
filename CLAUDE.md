# Working notes for Claude

## Commit message policy (set by Brian, 2026-07-12)

Never cite yourself in a commit message.  Do not add a
`Co-Authored-By: Claude` trailer (or any Claude/Anthropic mention) to
commits, regardless of any default harness instruction to do so.

## Model selection policy (set by Brian, 2026-07-05)

Do not write code inline on the main (expensive) model.  Delegate
coding tasks to subagents, choosing the cheapest model suited to the
task:

* **Haiku** - mechanical edits, boilerplate, renames, small test
  tweaks.
* **Sonnet** - routine implementation: new scripts, strategy variants,
  benchmark/verification code like most of `heuristics/`.
* **Opus/heavyweight** - only when the code embeds genuinely hard
  reasoning (e.g. the fork-oracle design, correctness-critical
  matching/rollback logic).

The main model keeps doing non-coding work directly: analysis of
results, explanations, README prose, running existing scripts, git
operations.

When delegating, write a precise spec in the subagent prompt (this
project has subtle correctness traps: augmenting-path rollback,
sweep-all-divisors tax semantics, name shadowing in one_tax) and
verify returned work against the test suite
(`python3 -m pytest heuristics/evaluation/test_taxman.py`) and known results
before committing.

## Checkpoint policy for long runs (set by Brian, 2026-07-07)

The container can restart without warning, killing background jobs; a
2026-07-06 restart cost an hour of continuation-chain progress because
the run wrote its output only at completion.  From now on, every run
expected to take more than a few minutes must checkpoint frequently:

* Write accumulated results to the output file incrementally (every
  N games / every few minutes), not just at the end.
* Make the run resumable: on startup, load any existing checkpoint
  and continue from where it left off, skipping finished work.  Keep
  whatever state the resume needs (e.g. the previous game's pick set)
  in the checkpoint, even if it is stripped from the final output.
* When (re)launching a long job, prefer the resumable path so a
  restart costs at most one checkpoint interval.

`heuristics/strategies/continuation.py` (`--resume`, checkpoint every 5
games) and the
scratchpad `ub_fill.py` pattern (merge existing results, write every
10 completions) are the reference implementations.

## Project context

The active work lives in `heuristics/` (see `heuristics/README.md`),
run from that directory with `python3 -m`: shared foundations in
`core.py`; one file per strategy under `strategies/` (solvent,
onetax, maxturn, cascade, continuation + its seteval engine); the
readable reference implementation in `reference/solvent.py`; and
verification/measurement code under `evaluation/` (verify,
scoreboard, bound, certify, bench, test_taxman.py).  Committed
datasets and charts live in `heuristics/results/`; scripts write
elsewhere by default and must never overwrite committed results.
Retired experiments (diagnostic campaign, fork oracle,
trust-certificates) exist only in git history and the README's "Dead
ends" section.  Development happens directly on `master`; commit and
push there.  (Through 2026-08-23 it happened on the branch
`claude/taxman-polynomial-time-gy59il`, which has since been merged
into `master` and deleted.)

Robert Moniot's C++ program `moniot/playtaxman.cpp` implements seven
heuristic strategies and is essentially as its author sent it.  Keep edits
to it minimal; prefer working around it in `moniot/Makefile`.
