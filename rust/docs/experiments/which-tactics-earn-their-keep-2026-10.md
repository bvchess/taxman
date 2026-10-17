# Which tactics earn their keep (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

*Question.*  The owner's: a tactic that is unproven and does not pay has no reason to stay.  Which tactics and
alternatives give meaningful acceleration in the case that matters, long strict chains at the frontier (n of 1200 to
1400 now, more on a 192-core machine, each game built from the one before, as `results/strict.jsonl` is), and which
should be recorded here and removed?  And what support for old file formats can go with them?  Strict is to be the
default, and a doubtful tactic that stays must be switched on explicitly.  This entry measures and recommends; the
removals are a later step.

*How it was measured.*  The solver code of commit 243af9b, which made `results/strict.jsonl` as it was then (unchanged
to 72601a2).  Searches are the ladder's proof-only ones on one thread (`ladder --threads 1 --seed-opt --known
results/strict.jsonl --seconds S [--tactic …] N`), whose node counts are exact, with the process's cycles from
`/usr/bin/time -l`, two at a time on the laptop while another agent measured cycles on it, so cycles are good to a few
percent.  What `reuse_prev` and `revise_freedoms` do was worked out from setups alone, with no search, by a scratch
example (not kept):

* `reuse_prev`: for every n of 2..1300 that is neither prime nor trivial, `accelerate::reuse_prev` with the strict
  chain's game of n − 1, its game compared with n's line in `strict.jsonl` (score, moves, proof, and
  `seconds.search_all_runs`); and a chain simulated with the tactic on, in the solver's order: prime, trivial,
  `reuse_prev`, else a repeat of the latest of the 8 board sizes before n that set a search up and poses the same
  problem (compared whole, as `examples/repeats.rs` does), else a search, with the derived games as n − 1 where it
  accepted.
* `revise_freedoms`: `Board::prepare` with the tactic on and off, one thread, for each searched or repeated game of
  1232..1300: what revising did, how long it took, and whether the two problems differ.

## `reuse_prev`: nothing left to save

With the strict chain's games as n − 1 it accepts 670 games of 2..1300, and the strict chain settles every one of them
without a search of any length:

| what the strict chain did with them | games | at the ceiling n + score(n − 1) | their search seconds |
|---|---|---|---|
| a repeat (`same_search`) | 545 | 300 | 0 |
| the cap, reached before the first node | 115 | 115 | 1.89 |
| an exhausted search | 10 | 0 | 1.92 |

The ten searches are of 51, 111, 119, 141, 183, 185 and 741 (each under 0.03 s), 1083 (0.11 s), 1183 (0.14 s) and
1209 (1.64 s, the only one over a second); the other 406 searches of the chain it rejects.  So a rerun of the chain
with the tactic on would save at most 3.8 s of its 1052 s of search (0.36%): 0.6 s of 51 s in 1..1000, 2.7 s of 188 s
in 1001..1231, and 0.5 s of 813 s in 1232..1300, all of it there the setups of seven proofs by the cap.  The simulated
chain agrees: 670 derived, 406 searched and no repeat at all, since the tactic comes first and takes every game whose
problem is its predecessor's (as `regress` showed in the same-search round), and no game that the strict chain repeats
would have to be searched instead.  No derived game differs from the strict chain's, in its score or in a single move,
so there is no counterexample either.  255 of the 670 are accepted below the ceiling, where only the conjecture says
that they are optimal; 245 of those are repeats, proven.  What the tactic once saved, the cap with the seed (every
game at the ceiling) and `same_search` (the rest) now prove.

## `revise_freedoms`: the same trees

Its premise is false for the whole frame game ([n =
833](../../prover/README.md#6-does-an-optimal-game-play-the-most-moves), `tests/soundness.rs`), and the owner decided to
remove it; measuring stopped there.  Measured before that: in every one of the 60 searched or repeated games of
1232..1300 it searches the two-frame game (282 to 2,238 nodes, 16 to 27 ms, 1.2 s for the 60) and lowers frame 0's
freedoms by 4 to 6 and `max_total` by 0 to 5, so every problem changes; yet seven frontier proofs have the same trees
with it on and off, node for node:

| proof | nodes, on and off | G cycles, on | G cycles, off |
|---|---|---|---|
| 1215 | 1,111,451 | 32.6 | 32.7 |
| 1225 | 2,178,362 | 86.8 | 86.7 |
| 1240 | 496,460 | 18.7 | 18.7 |
| 1254 | 4,830,651 | 149.9 | 149.5 |
| 1272 | 4,341,863 | 129.2 | 128.8 |
| 1287 | 20,972,332 | 803.8 | 803.5 |
| 1300 | 28,024,916 | 985.0 | 982.2 |

The lower count of promotions and frame 0's lower freedoms cut nothing that the matching bound and the conflicts do not
cut first.  In the chain the tactic adds about 20 ms of setup a game and a taint, nothing else.

## The alternatives that are off

The measurements that retired `preclusions`, `match_bound=off` and the other branchings predate the Lagrangian bound,
so they were repeated with it (one thread, proof only, the strict tactics with the one named changed; a limit of
180 s):

| proof | tactics | nodes | search s | G cycles | against the strict tactics |
|---|---|---|---|---|---|
| 1184 | strict | 52,259 | 0.47 | 1.6 | |
| 1184 | `preclusions=on` | 54,883 | 0.52, and 3.45 building the table | 12.5 | nodes +5%, cycles ×7.7 |
| 1215 | strict | 1,111,451 | 10.6 | 32.6 | |
| 1215 | `preclusions=on` | 1,569,468 | 14.6, and 4.46 building the table | 58.7 | nodes +41%, cycles +80% |
| 1215 | `branching=conflict-largest` | 9,098,354 | 66.6 | 204.8 | nodes ×8.2, cycles ×6.3 |
| 1215 | `branching=conflict-smallest` | more than 21.5 M | stopped at 180 | 554.9 | more than ×17 |
| 1215 | `branching=largest` | more than 32.0 M | stopped at 180 | 562.0 | more than ×17 |
| 1215 | `match_bound=off` | more than 67.7 M | stopped at 180 | 558.8 | more than ×17 |
| 1215 | `match_refine=0` | 1,123,153 | 9.3 | 28.7 | nodes +1%, cycles −12% |

The branchings other than `conflict-strong` lose more than before because the forced members (the matching's and the
Lagrangian bound's, tactic `fix_forced`) work only with it; without `match_bound` nothing of the last two rounds works
at all.  `legacy_bound` needs no measurement: it changes the runner-up of `blameGame`, which the default search never
computes, so it acts only with `match_bound=off`, and there it is unsound.  The last row is the open question of the
Lagrangian round, whether refinement still pays (there 0 to 26% fewer cycles without it on four proofs); its default
needs the bench set and the small games before it moves.

## The proven tactics' switches

None of the proven tactics has code of its own for the off position worth the name: each is a condition or an
`Option` that the on position needs anyway.  A switch costs a slot in `Tactics`, a bit of the checkpoint header, and
its share of the tests (each variant of `tests/solver.rs`'s `every_tactic_gives_the_same_scores` solves 1..300 with
`force_search`; `tests/common`'s variants and `tests/soundness.rs`'s configurations; tripwire rows).

| switch | code only for the off position | tests that use the off position |
|---|---|---|
| `same_search` | one condition (`solver::search_game`) | `tests/same_search.rs` (chains with it off are the reference), `derivations_can_be_switched_off` |
| `lagrange` | none (an `Option`, `None` without `nogoods` too) | 5 tripwire rows (the trees of the build before it), the probe's `--tactic lagrange=off` fingerprint, a variant |
| `fix_forced` | the `forcing` flag: about five conditions in `searcher.rs` and `refine.rs` | 6 tripwire rows, variants and soundness configurations |
| `nogoods` | none (the table is an `Option`) | 6 tripwire rows, variants and configurations |
| `forget_nogoods` | none (the period is an `Option`) | configurations (never, every step, every third), a variant |
| `prev_cap`, `seed_incumbent` | one filter each (`solver::search_game`); the ladder's `--no-cap` and `--no-seed` | a variant each; the standalone re-solves of `bench/curve/calib.jsonl` used both flags |
| `prime_shortcut` | one condition (`solver`) | `derivations_can_be_switched_off` |
| `match_refine` (0 to 7) | the early return of a child's check at level 0 (three lines); `Refiner::top` takes any depth; `MAX_MATCH_REFINE` and 3 header bits | 6 tripwire rows (0 and 3), variants and configurations |

## Recommendation

| tactic or alternative | grade | measured benefit in the frontier chain | recommendation | what removal deletes (details below) |
|---|---|---|---|---|
| `reuse_prev` (on by default) | unproven | 3.8 s of the strict chain's 1052 s of search (0.36%), 0.5 s of 813 s in 1232..1300; every game it accepts is settled anyway by the cap or a repeat | remove | `accelerate::reuse_prev`, method `reuse` and proof `reuse_prev`, the Java acceleration fixtures |
| `revise_freedoms` (on by default) | unproven, premise refuted at n = 833 | none: the same trees in seven frontier proofs, 20 ms of setup a game | remove (premise refuted at n = 833) | `Revise` and the two-frame search, `setup.revise` and `seconds.revise`, frame 0 in the checkpoint header |
| `legacy_bound` (off) | unsound | none: it acts only with `match_bound=off` | remove | `RunnerUp::Legacy` and its taint |
| `preclusions` (off) | sound | a loss: 1215 +80% cycles, 4.5 s to build the table | remove | `bounds::Preclusions`, `seconds.preclusions`, `stats.precluded` |
| `match_bound=off` (the Java's search) | sound | a loss: 1215 unfinished at 180 s, more than 17 times the default | remove | `blameGame` and the promotability checks, frame certificates, `stats.upper_bounds`, `can_promotes`, `certificates` |
| `branching=largest` | sound | a loss: more than ×17 | remove | the tactic `branching` (with the next two) |
| `branching=conflict-smallest` | sound | a loss: more than ×17 | remove | (with `branching`) |
| `branching=conflict-largest` | sound | a loss: ×6.3 | remove | (with `branching`) |
| `same_search`, `lagrange`, `fix_forced`, `nogoods`, `forget_nogoods`, `prev_cap`, `seed_incumbent`, `prime_shortcut`, `match_refine` | proven or sound | on by default, the frontier's speed (earlier rounds); their off positions are the baselines | keep, as switches for A/B measurements | nothing |

With `reuse_prev`, `revise_freedoms` and `legacy_bound` gone no unproven tactic is left, so every run is strict and
there is nothing doubtful to switch on: `--strict`, `tactics::UNPROVEN`, `Tactics::strict`, `Tactics::unproven_on`,
the line's `unproven_tactics_on` and the rules of a game's own taint (`results::own_taint`, `own_taint` in
`tools/check_results.py`) go, with their tests (`strict_turns_off_exactly_the_unproven_tactics`,
`a_game_made_with_unproven_tactics_on_but_not_used_is_clean`, the `--strict` cases of `tests/cli.rs`,
`design_grades_the_unproven_tactics_as_the_code_lists_them`), and a taint can then only be inherited from
`optimal.json` (whose games 1..1000 the Java made with those tactics on, so `--allow-tainted-prev` stays).  Of
`SearchTactics` there remain `match_refine`, `fix_forced`, `nogoods`, `forget_nogoods` and `lagrange`.

## What each removal deletes

* **`reuse_prev`.**  `accelerate::reuse_prev` (and the module comment's half on it); step 4 of `solver::solve_one`;
  `Method::Reuse` and `Proof::ReusePrev` with their rules in `src/results.rs` and `tools/check_results.py`;
  `FrameColumns::estimate_max_promotions` (its only user, with its check against the Java's `estimateMaxPromotions` in
  `tests/golden_hives.rs`) and the frames test `the_frames_candidates_are_the_plain_apiarys` (it guards the
  tactic's candidates); `examples/sacrifice.rs`'s `--select reuse` (the theorem of `src/sacrifice.rs` stays; its module
  comment's motivation changes); tests: `acceleration_matches_the_java_beyond_the_golden_data` with
  `testdata/accel_java.txt`, the `reuse_prev` half of `acceleration_matches_java` (its prime half can stay), the Java's
  count of 146 derived games in `a_range_reproduces_optimal_json_and_the_javas_counts`, and the tests in
  `tests/integrity.rs` and `tests/cli.rs` that take a `reuse_prev` game as their tainted game (they can take one built
  on `optimal.json`); DESIGN.md's rows and step 4.
* **`revise_freedoms`.**  In `src/search/setup.rs`: `revise_freedoms()`, the enum `Revise` with its eight values
  (`Off`, `NotNeeded`, `GreedyIsMax`, `Unchanged`, `Searched`, `Stopped`, `FromCheckpoint`, `ResumedAsBuilt`),
  `Revise::parse`, `frame_zero` and `sets_frame_zero`, `resumed_root`'s revised branch, `Prepared::unrevised` and
  `PrepareTimings::revise`; `FrameSet::with_frame_zero_freedoms` and `front_pair` (no other user); the checkpoint
  header's `frame0_freedoms` and tactic bit 2; the `revise` argument of `board::search_problem` and
  `search::problem_of`, and of the solver's `Earlier`; the solver's handling of a stopped revision; the line's
  `setup.revise` and `seconds.revise`, with their contradiction checks and taint rule in both verifiers; tests:
  `a_game_resumed_with_revised_freedoms_rests_on_revise_freedoms` and
  `a_game_resumed_with_the_built_freedoms_rests_on_nothing_unproven` (`tests/solver.rs`),
  `a_limit_also_stops_the_search_of_revise_freedoms` and the revised half of the frame-0 forgeries
  (`tests/robustness.rs`), the `setup.revise` cases of `tests/results.rs`, the revised half of the perturbed games of
  `tests/soundness.rs` (`Game::revised` and the exclusion of the games that break the conjecture), the Java's
  `revisedFrame0Freedoms` and `maxTotalPromotions` in `search/tests.rs`'s `check_against_java` (its
  `maxPromotionsUnrevised` takes their place); the `revise_freedoms=off` prefix of the tripwire rows of 1133 (the rows
  of 480 to 936 were taken with it on and should not move, since it changed no tree measured here; if one does, it is
  re-baselined).  The test of n = 833 stays as a fact about the frame model.
* **`legacy_bound`.**  `RunnerUp::Legacy` and `SearchTactics::rule`, the `legacy_bound` argument of `own_taint` and
  its rule in `tools/check_results.py`, header bit 1, the legacy comparisons of `bounds.rs`'s `upper_bound_is_sound`
  and the `legacy_bound` cases of `search/tests.rs`; all of `RunnerUp` goes with `match_bound=off`.
* **`preclusions`.**  `bounds::Preclusions` with `explore_preclusions` and `below`, the preclusion step of
  `Bounds::new_candidate_list_after`, the table's build in `prepare_state` and `PrepareTimings::preclusions`, the
  line's `seconds.preclusions` and `stats.precluded`, header bit 0; tests: `preclusions_do_not_depend_on_threads`,
  `preclusions_match_the_java_procedure` (the Java's `findPreclusions`) and `preclusion_lookup` in `bounds.rs`, the
  tripwire row of 480, a variant and a soundness configuration, the case of `tests/robustness.rs` that resumes with it
  changed (another tactic serves), and the preclusions of the random walks of `matchbound.rs`'s tests.
* **`match_bound=off`.**  The `blameGame` path of `bounds.rs` (`Bounds::upper_bound`, `reaches`, `largest_that_fit`,
  `upper_bound`, `new_candidate_list`'s promotability checks and `can_promote_certified`, `first_failure`, `Blame`,
  `RunnerUp`: about 300 lines, and about 300 of tests: `first_failure_matches_promoting_one_by_one`,
  `upper_bound_is_sound`, `candidate_list_check_limit`, the bound half of `greedy_and_bound_at_the_root_of_85`, the
  ignored benches); the frame certificates that only those checks use (`Certificate` and its recorder in
  `model/peel.rs`, `certify_using`, `winnable_with_move_using` and `winnable_without_factor_using` in
  `model/promote.rs`, and their model test); the searcher's `cut::BLAME` tests and the optional matching graph (it
  becomes mandatory); `search::Diagnostics`' `blameGame` gaps; the line's `stats.upper_bounds`, `can_promotes` and
  `certificates`; header bit 3; `matchbound.rs`'s ignored `matchbound_pruning` study against `blameGame`; a variant and
  a soundness configuration; DESIGN.md's account of the Java's loop as the baseline (section 4).  What stays of
  `bounds.rs`: `max_additional`, `greedy` (the Java's, checked against its golden data) and the narrowing of the
  children's candidates with its scratch space.
* **The other branchings.**  The tactic `branching` itself: the enum `Branching`, its names and parser, the match in
  `Stepper::choose_branch` (the fallback to the largest candidate where there is no conflict stays), `wants_conflict`,
  `fix_forced`'s dependence on `conflict-strong`, header bits 7 and 8; tests: two tripwire rows of 600, three variants,
  four soundness configurations, the branching cases of `src/tactics.rs`.

## Old file formats

Everything that exists only to read what older builds wrote, and what deleting it takes with it:

* **Results lines of schemas 1 to 3.**  In `src/results.rs`, `parse_line`'s schema branches (schema 1: no `schema`
  field, `reached_cap` and `in_the_bag` derived, no proof, taint or `prev` record), `taint_by_schema` (before schema
  3 a proof by the cap counted the game's own taint), the `FromCheckpoint` exception of schemas 1 and 2 in
  `contradiction`, the optional digest of lines before schema 4, and the module comment's account of them; the same in
  `tools/check_results.py` (its `schema` branches, `legacy` `FromCheckpoint`, the schema-1 cap and `in_the_bag`).  In
  the solver, the problems set up again stay (nothing rests on a digest alone), but the optional digest of
  `Earlier` and the schema-3 reason given for `SAME_SEARCH_WINDOW` go.  Tests:
  `a_chain_from_schema_3_lines_repeats_their_searches` with `testdata/strict_schema3_1_90.jsonl` (124 KB); the schema-2
  cases of `lines_whose_fields_contradict_each_other_are_refused_by_both_verifiers`; the schema-1 counts of
  `the_committed_results_pass_the_verifier`.
* **The committed schema-1 files.**  `results/default.jsonl` (1001..1231, the first run past 1000, its games resting on
  `reuse_prev` and `revise_freedoms`) has nothing left to show that `strict.jsonl` does not (its scores, and its moves
  but for 1221..1224, are `strict.jsonl`'s): delete it (git keeps it).  Its readers: `tests/results.rs`'s pinned
  counts, `tests/golden_hives.rs` (the games behind `accel_java.txt`, which goes with `reuse_prev`), the default
  `--results` of `evidence/src/bin/exact.rs` and `moves.rs` (a fallback past `optimal.json`, which now reaches 1300;
  point it at `strict.jsonl`), `COMMITTED_RESULTS` in `tools/cloud/env.sh` (used by `setup.sh` and `verify.sh`), and
  the text of README.md, `results/README.md`, DESIGN.md and RUNBOOK.md.  `bench/curve/chain.jsonl` (the cost-curve
  study's strict chain 1001..1231 of commit e710d16) is that study's raw data: `analysis.py` reads its seconds, and
  `--known` (`Optimal::with_results`) and `tools/cloud/scaling.sh` only its scores and moves, which work with any
  schema.  Keep it as data, like `calib.jsonl`, and take it out of what the verifiers check (`tests/results.rs`,
  `COMMITTED_RESULTS`, the README's and `results/README.md`'s commands); or delete it with the study's scripts
  reading it from git.  The notebook's entries that cite these files stay as history.
* **`strict.jsonl` itself is schema 4.**  The removals above change the line (`setup.revise`, `seconds.revise`,
  `seconds.preclusions`, the `stats` of the Java's search, `unproven_tactics_on`, the switches in `tactic_switches`),
  which makes schema 5 and schema 4 an old format.  Either the verifiers keep reading schema 4, or `strict.jsonl` is
  replaced by a run of the new build (about 18 minutes on four threads of the laptop, as for commit 2487671), after
  which only schema 5 is read.  The second leaves no old format at all.
* **Checkpoints.**  `src/search/header.rs` refuses version 2 (`TXS2`) with a message of its own, and records the
  tactics added after the format (`fix_forced`, `nogoods`, `forget_nogoods`, `lagrange`) by the bits of their
  non-default values so that older checkpoints read back with them on; `search::checkpoint_width` and
  `solver::solve_one`'s widening resume a checkpoint at the set width an older build chose (32 for 1024..2047).  The
  removals change the header anyway (the bits of the removed tactics, `frame0_freedoms`), so a version 4 with the
  remaining tactics recorded plainly can refuse everything older with one message.  Fixtures:
  `testdata/n1133_after_1000_nodes.ckpt` (W = 32, version 3, made before `lagrange`; `tests/wide.rs`'s
  `a_committed_checkpoint_of_a_w32_game_resumes`) is made again at the width the build gives 1133 (W = 20) and keeps
  its role as the format's tripwire; the width widening goes with the old width unless set widths are expected to
  change again; `testdata/perf/n0936.window.ckpt` and `n1215.window.ckpt` (made before `lagrange`, read back with it
  on) are made again with `perf make-window`, with a new fingerprint for DESIGN.md.  The `.nodes` fixtures and
  `testdata/n1215_deep_units.txt` are lists of numbers, not checkpoints, and stay; the engine's own format
  (`TXCKPT01`, version 1) has no older version to read.
* **The Java's golden data.**  What checks the frame builder and the setup stays, since the default search starts from
  it: `testdata/golden` (318 boards to 760) and `testdata/golden_wide` (11 boards to 2048) with `apiary_matches_java`,
  `frames_match_java` and `wide_boards_match_the_java` (hives, frames, the reduced frame set, the unrevised
  `max_total`, the greedy set), `testdata/games_java.txt` with
  `promotion_sets_become_the_java_game_beyond_the_golden_data` and `promotions_become_the_java_game` (the hives turning
  promotions into the Java's game), `bounds.rs`'s `golden_greedy`, `search/tests.rs`'s
  `golden_games_up_to_400_match_the_java` (the Java's promotion sums), the Java quirks of `hives.rs`, and
  `tools/golden/export.sh`.  What reproduces the Java's tactics goes with them:
  `testdata/accel_java.txt` and the `reuse_prev` checks (with `reuse_prev`), the revised freedoms (with
  `revise_freedoms`), `findPreclusions` (with `preclusions`), and `blameGame` with its runner-up (with
  `match_bound=off` and `legacy_bound`).  The golden files' fields that then nobody reads (`searchManager`'s
  `revisedFrame0Freedoms`, `solve.accelerated` if the prime half goes too) can stay as exported.

*Decision.*  The owner took the recommendation and went further; the removals are commits f72c332, 8a6df90,
49ec778 and a2bc798 (on 4372b36 by the merge 39ed18b), about 4,800 lines of code removed and 1,650 added (`src/`,
`tests/`, `examples/`, `tools/`, `evidence/` and the study scripts; 3,600 and 990 of them in `src/`):

* *Tactics.*  `reuse_prev`, `revise_freedoms` (premise refuted at n = 833), `legacy_bound`, `preclusions`,
  `match_bound=off` with `blameGame` and the counters only they used, and the tactic `branching` with its three
  losing values are gone, with their tests, tripwire rows and fixtures (`testdata/accel_java.txt`).  The nine proven or
  sound tactics of the table stay as switches.
* *No strict runs.*  With no unproven tactic left there is one kind of run: `--strict`, `UNPROVEN`, `Tactics::strict`,
  `unproven_tactics_on` and the rules of a game's own taint are gone, and so are `STRICT_ARGS` in `tools/cloud`.
* *Assumptions instead of taint.*  A range takes the game of n − 1 from the run or from the `--out` file and otherwise
  solves its first game standalone; `--use-prev` takes it from optimal.json.  A result whose proof rests on such a game
  records `"assumes": ["the game of 1300 in optimal.json is optimal"]` (say), carried down the chain as taint was, and
  `[]` otherwise.  Nothing refuses a game for what it assumes (`--allow-tainted-prev` is gone), and `extend-optimal`
  appends such games: they assume nothing the table does not stand for.
* *optimal.json is built in.*  The binary carries the table it was compiled with (`include_str!`; a rebuild picks up a
  changed file) and confirms every score against it, a difference being a hard error naming the game: `--optimal FILE`,
  `--optimal none`, `TAXMAN_OPTIMAL` and `allow_optimal_mismatch` are gone.  Tests that need another table use the
  library (`solver::solve_range_each_with`, `results::read_results`); `verify-optimal [FILE]` and `extend-optimal
  RESULTS --out FILE [--table FILE]` still take files.
* *Old formats.*  Results lines are schema 5 (`assumes`, `prev.assumes`, `same_search.assumes`, `options.use_prev`; no
  `prev_source`, `setup.revise`, `seconds.revise` or `seconds.preclusions`, `unproven_tactics_on`, `optimal_file` or
  `allow_optimal_mismatch`), and both verifiers read schema 5 only; `results/default.jsonl` and
  `testdata/strict_schema3_1_90.jsonl` are deleted (`git show f72c332:rust/results/default.jsonl`), and
  `bench/curve/chain.jsonl` stays as the cost-curve study's data, which no verifier reads.  The checkpoint header is
  version 4 (no frame 0, the tactics recorded plainly), every other version is refused with one message, and the
  widening to an older build's width is gone; `testdata/n1133_after_1000_nodes.ckpt` (now W = 20) and the probe's
  windows were made again, the probe's fingerprint now 7e8e724717d26545 (5b83bbebf0bb2899 with `lagrange=off`).
* *Left for the owner.*  `results/strict.jsonl` is schema 4, so until it is made again by this build (the chain from
  n = 1, about 18 minutes on four threads) neither verifier reads it: `tests/results.rs`'s
  `the_committed_results_pass_the_verifier` skips it, saying so, and `tools/cloud/verify.sh`, `setup.sh`'s verifier
  smoke check, `tools/profile/profile.sh` and the README's commands that extend a copy of it wait for the rerun.

## Later (2026-10): the program removed from master

The examples `sacrifice` and `repeats` were removed from master on 2026-10-17.  They build at commit 8753189:
`git checkout 8753189`, then `cargo build --release --example sacrifice --example repeats` in `rust/`.
