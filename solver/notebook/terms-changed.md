# Terms changed (2026-10)

Part of the [lab notebook](README.md).

The dated entries below use the words of their time; they were not rewritten.  In 2026-10 the project settled on the
wiki's words, and the code, DESIGN.md, the README and the other guides, and the register below, use them.  An
old word in an entry maps like this (the names of fields and files did not change: results keys, checkpoint headers,
tactic names, data file fields, golden data keys, `results/strict.jsonl`):

| in the entries | the word that replaced it |
|---|---|
| hive, apiary, level (of hives) | frame; the frames that get the same number are merged into one |
| the types `Hive`, `Apiary`, `FrameColumn` and `FrameColumns`, the modules `hives` and `frames` (two files) | `frames::Frame` (a frame as setup forms it; the search takes it as `model::Frame`), `BoardFrames`, `SearchFrames`; one module, `src/frames/` |
| `hive_index_of`, `downstream_hives` and `upstream_hives`, `hive_promotion_candidates`, `hive_solution`, `free_factor_count`, `has_free_factors`, `levels_of_free_hives` | `frame_index_of`, `frames_in_front` and `frames_behind`, `promotion_candidates_of`, `solution_of`, `freedoms`, `is_closed`, `numbered_frames` |
| `model::Model`, `results::Certificate`, `tests/golden_hives.rs` | `model::FrameGraph`, `results::CeilingProof`, `tests/golden_frames.rs` |
| downstream / upstream, below / above, bottom / top | in front of / behind, front / back (frame 0 is the front) |
| free factor count, spare factors | freedoms |
| feasible (a set of promotions) | promotable |
| cap, the cap on the promotion sum | the ceiling (`n + score(n − 1)`; as a promotion sum, minus `in_the_bag`); `cap`, `reached_cap`, `prev_cap` stay the names of fields, a proof kind and a tactic |
| the cap on the number of promotions (`max_total`) | the promotion limit, the most promotions the freedoms allow (`max_total` in the code) |
| core (what a peel cannot drop in a frame) | residual (what the code's peel, the theory's dismantling, leaves; the theory's *remainder* is what moves of its peel leave) |
| the evidence crate, `rust/evidence/`, `evidence.sh`; the model-free search, checker, proofs, bound | the prover, `solver/noframe/`, `solver/tools/cloud/prover.sh`; the prover's search and proofs; a bound on the whole board |
| the matching bound of the model-free search, chain relation, prime relation | the Franklín–Moniot bound, the ceiling, the prime formula |
| the Rust solver, the exact solver, the main solver | the frame solver ("the solver" for short; the prover stays the prover) |
| strict chain, strict runs | the committed chain (the file keeps the name `results/strict.jsonl`) |
| frontier (the largest n solved in a row, or the hardest games) | the end of the chain, the hardest games |
| cores (CPU) | CPUs, threads |
| selection, pick | move |
| British spellings (neighbour, labelled, analyse, behaviour, centre, colour, catalogue, optimise, ...) | the American spelling (neighbor, labeled, analyze, behavior, center, color, catalog, optimize); the docs test lists the words, and the names in committed data (`neighbours_cpu_s`, `cover_relabelled`, the seed `"neighbour"`) are as written |
| board size | game n, the game of n, or n |
| the optimum (a noun) | opt(n), the best score; P\*, the best promotion sum |
| the strong bound (`wall choice`) | the Lagrangian bound over the nogoods, with the GF(2) bound inside, refitted at the node |
| the nogood bound | the Lagrangian bound over the nogoods |
| the A1 bound, the fields bound | the multi-field bound (the agenda's A1) |
| checker | the referee (of one game); a verifier (of a whole results or bounds file) |
| EXPERIMENTS, EXPERIMENTS.md | the notebook (this page and its entries) |

The last seven rows are the names that [the glossary](../../docs/GLOSSARY.md) replaced on 2026-10-18; the docs test
holds the guides to them.

## Removed programs

The example programs of finished experiments (`fields`, `matroid`, `dominance`, `slope`, `repeats`, `promotions`,
`diff`, `hunt` and `sacrifice`) were removed from master on 2026-10-17, with the files of `examples/common/` that only
they used and `solver/tools/cloud/hunt.sh`.  An entry that used one ends with a section `## Later (2026-10)` naming the
commit where it builds (8753189); the register and the entries keep the paths and commands of their time.
The commands `chain`, `sound`, `finders`, `nogoods`, `target` and `relaxed` of the example `wall` were removed on
2026-10-18, with the code only they used (`choice-prepare` and `choice` stay); their entries end the same way, and name
the commit 9d9dae3.  The large neighborhood search (`examples/common/lns.rs`) and the seeded generator
(`examples/common/rng.rs`) moved into the library on 2026-10-21 as `src/search/finder.rs` and `src/rng.rs`; the
entries before that date name the old paths.  The study tools (`tune`, `extended`, `structure`, `finder` and
`wall`) and the fixtures `solver/frame/testdata/fits/` were removed on 2026-10-29; they build at commit 3ca11bf, and the
entries that used them keep the paths and commands of their time.  The sacrifice bound (the library module
`solver/frame/src/sacrifice.rs`, with `sacrifice::hunt`) and the punctured boards it used (`Tables::punctured`,
`game::replay_punctured`, the maximal-factor rule of `brute::solve` and the hole cases of `frames/`) were removed the
same day, and the test of `tests/wide.rs` that took its upper bound for game 1133 from the sacrifice bound reads the
prover's proof instead; they build at commit 3ca11bf too.

The speed tools were removed on 2026-10-30: the command `taxman bench` (its history file,
`solver/frame/bench/history.jsonl`, stays as the record of the benchmarks of 2026-10), the example `ladder`
(`solver/frame/examples/ladder.rs`), the scaling sweep `solver/tools/cloud/scaling.sh` and the profile tools
`solver/tools/profile/` (`profile.sh` and `attribute.py`), and the profile-guided build `solver/tools/pgo.sh` (it gained
0 to 2% on the M1 and no chain ran on it).  They build at commit 3ca11bf, and the entries that used them
keep the paths and commands of their time.
