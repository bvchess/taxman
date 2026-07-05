# Working notes for Claude

## Model selection policy (set by Brian, 2026-07-05)

Do not write code inline on the main (expensive) model.  Delegate
coding tasks to subagents, choosing the cheapest model suited to the
task:

* **Haiku** - mechanical edits, boilerplate, renames, small test
  tweaks.
* **Sonnet** - routine implementation: new scripts, strategy variants,
  benchmark/verification code like most of `mini/`.
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
(`python3 -m pytest mini/test_taxman_mini.py`) and known results
before committing.

## Project context

The active work lives in `mini/` (see `mini/README.md`): a clean
Python implementation testing the theory that the >N/2 selections of
an optimal Taxman game are computable in polynomial time, plus
O(n^2)-class full-game strategies measured against
`src/main/resources/optimal.json`.  Development happens on the branch
`claude/taxman-polynomial-time-gy59il`; commit and push there.
