# Working notes for Claude

## Model selection policy (set by Brian, 2026-07-05)

Do not write code inline on the main (expensive) model.  Delegate
coding tasks to subagents, choosing the cheapest model suited to the
task:

* **Haiku** - mechanical edits, boilerplate, renames, small test
  tweaks.
* **Sonnet** - routine implementation: new scripts, strategy variants,
  benchmark/verification code like most of `polytime/`.
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
(`python3 -m pytest polytime/evaluation/test_taxman.py`) and known results
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

`polytime/strategies/continuation.py` (`--resume`, checkpoint every 5
games) and the
scratchpad `ub_fill.py` pattern (merge existing results, write every
10 completions) are the reference implementations.

## Project context

The active work lives in `polytime/` (see `polytime/README.md`),
run from that directory with `python3 -m`: shared foundations in
`core.py`; one file per strategy under `strategies/` (solvent,
onetax, maxturn, cascade, continuation + its seteval engine); the
readable reference implementation in `reference/solvent.py`; and
verification/measurement code under `evaluation/` (verify,
scoreboard, bound, certify, bench, test_taxman.py).  Committed
datasets and charts live in `polytime/results/`; scripts write
elsewhere by default and must never overwrite committed results.
Retired experiments (diagnostic campaign, fork oracle,
trust-certificates) exist only in git history and the README's "Dead
ends" section.  Development happens on the branch
`claude/taxman-polynomial-time-gy59il`; commit and push there.
