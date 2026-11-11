# The tests

Part of the [design](../DESIGN.md).

What the full suite checks, by subject.  A few tests sample what an exhaustive run would cover (every 18th golden game
for the peeling agreement, every sixth game above 511 for the root promotability of the frame builder's candidates,
every second line of the Java fixtures), and the comment on each says what the whole run costs.

* **Unit tests**, per module, include the Java's unit tests' cases (of the factor table, frames, and frame sets, for
  n = 85).  The helpers that the unit and integration tests share (temporary directories, a random generator) are one
  file, `tests/common/support.rs`.
* **Golden tests** (`tests/golden_frames.rs`, and unit tests in `src/search/tests.rs` and `src/bounds.rs`) check every
  stage of the port against the Java's answers in `testdata/golden/n*.json` ([its
  README](../../frame/testdata/golden/README.md) says where they came from): the setup's frames, the numbered and merged
  frames, the candidates, the reduced frame set, the greedy incumbent, the prime shortcut and the result of the solve.
  On eight small boards the Java's claims are not those of the theory's root problem, and the test lists where setup
  departs from them.  The searched golden games (94, up to 760) give the Java's promotion sums with the default tactics,
  and the tactic variants give the same answers on those up to 400.  Past 760 the Java's structure is fixed in
  `testdata/golden_wide/` (n = 511, 512, 1000, 1001, 1023, 1024, 1025, 1100, 1231, 2047 and 2048, checked at the width
  `with_width!` picks) and `games_java.txt` (the Java's moves on the board's frames, move for move, for 171 sets of
  promotions at 511–2049).
* **The board layer against the chain** (`tests/board_layer.rs`): for every 25th searched line of
  `results/optimal.jsonl.gz`, and the last, the moves that the game's promotions make and the digest of the board's search
  problem are the line's, which the golden data (up to 2049) cannot say of the games that matter.  The unit tests of
  `frames` check what the frames remember (which numbers each can take on, which sources `removeImpossible` has been
  through) against a fresh computation.
* **The frame model's exactness** ([the bottom-layers theorem (F4)](../../../docs/theory/frame-model.md#the-bottom-layers-theorem)
  and [the exactness theorem (X7)](../../../docs/theory/exactness.md#the-exactness-theorem);
  [frame-model.md](frame-model.md#what-optimal-means)):
  * `tests/model_exact.rs` decides playability on the whole board with an oracle that shares no code with `frames` or
    `model` (`src/playable.rs`, the peel, itself checked against the rules on every subset of the games to 20).  It
    checks both directions of X7 on every set of every game to 100 (to 120 ignored) and on random sets of games
    to 1300 (to 10239 ignored), and the near miss that only the candidate filter keeps out (n = 98).
  * `tests/bottom_layers.rs` checks F4 on every game of the committed chain and another run of
    its lines for 2176–2276 (`testdata/c8g.chain-2176.jsonl`), every optimal set the prover enumerated, and every
    optimal set of the games to 48.
  * The unit tests of `frames` check that `unzip`'s claims are its mandatory pairs, that `numbering` is what
    `build_frames` uses, that the frames lie in layers (F1), and that the numbering by layer is the Java's
    breadth-first numbering on every game to 400 and a sample to 10239.
  * `tests/canonical_root.rs` checks that setup builds Π_n: its U\*, its candidates and its root frames are those of
    `src/root_problem.rs`, which builds Π_n from the definitions with no code of `frames` or `model`, on every game to
    200 (to 1000 ignored).
  * The census of every game to 8191 was run once, by the example `structure` (removed since; it builds at commit
    3ca11bf, and its data are in
    [taxman-notebook/the-claims-of-unzip-in-the-extended-problem-2026-10](https://github.com/bvchess/taxman-notebook/tree/main/the-claims-of-unzip-in-the-extended-problem-2026-10)).
    It checked what X7's proof says of setup one game at a time, that Π⁺(∅) with the claims of `unzip` is Π_n (check H),
    and that setup builds Π_n (check I).
  * The census of every game from 8192 to 10239, the games of the widest set, is the ignored unit test
    `census_of_the_boards_past_8191` of `frames`, run in a release build in minutes on 8 threads.  It makes the
    checks of the first census but H, whose code the frame solver no longer has, and measures the frame counts and
    the GF(2) ranks there ([frame-model.md](frame-model.md#setup-in-the-code), the paragraph "The censuses").
* **Regression**: `taxman regress --to N` solves 1..N as one range and checks every score against the built-in table.
* **Tactic invariance**: games 1..300, forced to search, give identical scores with each tactic off or changed, and a
  checkpoint is refused when any of the search tactics differs.
* **Bound soundness**:
  * on small frame sets, the matching bound is never below the best completion computed exhaustively;
  * the incremental matching solver, as the dive drives it (candidates removed a few at a time), agrees with the
    reference min-cost flow and with a solver started afresh, and its states pass the optimality certificate;
  * the GF(2) bound (`src/gf2/tests.rs`, `src/gf2/meet.rs`): the intersection against brute force under every size
    limit, from every set it reports as a warm start, with members deleted, and its weight split's bounds without each
    member.  On the games up to 400 the bound is never below the exhaustive best completion, and the optimal
    promotions are independent in every frame.  Along random paths of games 60–1000 (promotions, deletions,
    backtracking) every level equals the one built afresh at its node, every member it calls forced is (the bound
    without it, solved afresh, is below what the node needs), and every tableau derived instead of built is the one a
    solve builds; along the same kind of paths the copies of every word count (1 to 5, padded) give the same results
    and do the same work.  Blocks of more than 255 members (five words) are checked against the greedy algorithm, one
    side free and the other random.  At the 600 nodes of the probe's fixtures (936 and 1215), completions promoted
    in random orders are independent in every frame of the node and no heavier than its bound (`tests/soundness.rs`);
  * conflicts and residuals are checked against promoting the sets;
  * the checks of the near-repeat cut (`src/search/near_repeat/tests.rs`): on constructed problems, each check failing
    alone where it must, among them a number that Π_M's model promotes but that is no candidate of Π_M (22 at n = 98),
    and the checks holding where the problems agree or Π_n's frames are harder; and at random nodes of the boards 30
    to 160 against each of the three boards before, every completion that can be promoted in Π_n where the checks hold
    can be in Π_M, by the model's own promotability;
  * the prime exchanges (`src/search/symmetry.rs`): the symmetric pairs against their definition (every number looked
    at, every pair of primes tried) on the games 2..=300 and a sample to 2200; the images of the optimal set and of
    random maximal promotable sets promotable and worth the formula; every optimal set keeping every inequality; on the
    games with at most 22 candidates, every promotable set so checked, and the best that keeps the inequalities worth
    P\*; and the rule's cuts and drops against its definition by enumeration (exact where the pairs are independent).
* **Soundness harness** (`tests/soundness.rs`): the search starts from an incumbent worth 1 or 5 less than P\* with an
  empty payload, so that any pruning that loses every best completion leaves the fake incumbent in place (which tests
  the `<` and `<=` boundaries too).  It runs:
  * every game 2..=300 under each of 24 configurations: 5 of the default, the GF(2) bound with the rule of symmetric
    pairs (without forced members or nogoods; forgetting every step; two threads), the GF(2) bound without the rule on
    one thread, two, and forgetting every step, 5 of conflict branching (at every check, whatever its gap, on one
    thread and two; off; and with the look at the last child of the search's own conflict, on one thread and two), 1
    without the near-repeat cut, 4 of the reuse of prices (conflict branching at every check without it, and under three
    other schedules of it), 4 of carried nogoods, and 2 without the warm start (one of them without the reuse of prices
    too); the others have both, as the default does;
  * the near-repeat cut in every search of the harness on real games, by one of the four games before (the latest whose
    optimum the handicapped incumbent reaches), and the games 404, 430 and 436, whose trees it changes, under every
    configuration with the cut and without it; on those three, a cut whose bound the incumbent never reaches cuts
    nothing and leaves the tree as it is without the cut, and a search that starts below the bound cuts once its
    incumbent has reached it;
  * the games 301..=500 (every tenth) with conflict branching at every check, with and without the look at the last
    child of the search's own conflict: the look changes the tree of some of them, and no answer;
  * every game 2..=600 with symmetric pairs, with the rule, with forced members and without (and the oracle without
    the frame model to 40); and units on the way to an optimal set (an optimal set less 30 to 70% of its members, games
    600..1000) with the rule;
  * the same for 500 perturbed frame sets (numbers dropped, a role flipped, moved a frame) with the truth found
    exhaustively; promoting a set at once against one by one in random orders; conflicts and nogoods on perturbed
    models;
  * searches interrupted every 1,000 nodes and resumed from a checkpoint with units spilling: game 992 in three
    configurations (game 1000 in a quick run);
  * units of real games at W = 20 to 64 (thinned so that the truth is found exhaustively) from a handicapped incumbent,
    and game 992 resumed every 1,024 steps at W = 20, 24 and 64 (game 1000 at W = 20 and 64 in a quick run), which
    carry the harness past the widths of the Java's table (n ≤ 1000).

  A search below the root with the rule of symmetric pairs is held to the best completion that keeps their
  inequalities (an exhaustive oracle of its own, which prunes by the same bound but shares no code with the rule), which
  it must reach; at the root that is P\* ([R2](../../../docs/theory/search-rules.md#the-prime-exchange-theorem)).
* **Node-count tripwires** (`tests/tripwires.rs`): a pruning tactic that stops working changes no answer, so no
  assertion on answers sees it (a mutant that ignored the result of `Nogoods::ruled_out` takes 27 to 67% more nodes).
  With one thread the tree is deterministic, so the test asserts the exact node count of a fixed game under fixed
  tactics: n = 480 to 1133, every tactic off or changed in turn, and the tree of game 800 with the searchers
  forgetting their nogoods every 200 steps.  The default gate, 300, changes the trees of only two of these games
  (800 and 1000), so the table `GATED` has the trees under the gates that make conflict branching act (the games 760
  and 1133 under a gate of 0, 1000 under 300), with the look and the reuse of prices off in turn, and the game 775
  under a gate of 1,200 with and without the warm start.  The
  table `NEAR_REPEATS` counts the trees of the games 768, 1083 and 1101, solved as the solver solves them from the
  committed moves for n − 1, with the near-repeat cut and without it, and checks that the line of each counts the nodes
  the cut cut (`stats.near_repeat_cut`) if it was armed, and has no such key if it was not.  The rows `symmetry=off`
  are the default's trees before the rule of symmetric pairs.

  **A change that alters the tree on purpose fails them.**  Run `cargo test --release --test tripwires -- --nocapture`
  (the quick tests check only the rows of trees of 250 nodes or fewer): it prints the rows as observed, in the layout of
  the table at the top of the file.  Check that the promotion sums (the answers) have not moved, paste the rows in, and
  update any node counts quoted in the docs.
* **Engine** (`tests/engine.rs`, `tests/robustness.rs`, `tests/stop_signal.rs`):
  * a toy problem with a known answer (knapsack, against dynamic programming), run with 1 and many threads, with a tiny
    `queue_mem_cap` to force spilling, and killed and resumed from checkpoints, gives the same answer.  Ignored tests do
    the same with 64 and 192 threads, checkpoints every 2 ms, and a panicking worker (`cargo test --release --test
    engine -- --ignored many_threads`), and draw random configurations through any number of resumes
    (`TXENGINE_STRESS=<n>`);
  * a tree with no pruning checks conservation exactly: under random configurations the nodes counted must equal the
    tree's size, so one unit lost or explored twice shows (`TXENGINE_CONSERVATION=<n>` for more);
  * robustness: an interval of zero finishes; limits too large for the clock are never reached; a deadline that has
    passed, or a stop signal received, before a run starts stops it before any search, with every unit in its
    checkpoint; spill segments of dead processes are removed; a locked checkpoint directory or results file is
    refused; a checkpoint of another game or format version, with other tactics, or with an incumbent that does not fit
    is refused; and
    the binary rejects bad times, survives a closed stdout and stderr, and stops on SIGINT and SIGTERM with a checkpoint
    it then resumes;
  * the control: a ceiling given before a run, or while it runs at the incumbent it has, ends it as optimal (with no
    search work if the start reaches it); offers from outside raise the incumbent; a stop through it checkpoints a run
    that resumes exactly; workers added, retired and down to none (suspended, then resumed,
    checkpointed, stopped, or ended by a ceiling) give the exact answer; and the conservation test runs with a
    controller that sets 0 to 4 workers every millisecond or so while the run checkpoints, spills and resumes;
  * the counters: capacity is wall × threads when the workers do not change, and a searcher that sleeps before its
    first and second polls shows in the rebuild and first-step times;
  * the time a run has spent: the workers' CPU time grows in the status of a running search and is whole in the counters
    at the end, no more than the workers' lifetimes; a run's account is its own (active throughout, never waiting)
    until the caller reports another through the control, which the checkpoint hooks and the outcome then carry.
* **The profile of a search** (`src/search/profile.rs`, games 800 and 1020): the same steps, counters and answer with
  it and without; every step counted once, on four threads with a checkpoint every millisecond and all the ids made,
  and through a resume past a block a crash left, behind a lost block, or with no profile; its `start`, `steps` and
  `end` lines carry the active and CPU time of the search.  Debug builds place every
  unit a searcher hands over.  The folder of `--profile`, like that of `--carried-dir`, is made when the run starts, and
  a folder that cannot be made fails the run before its first game (`tests/solver.rs`).
* **The window** (`tests/window.rs`; the policy's unit tests in `src/solver/window/allot.rs`):
  * chains 1..600 with windows of 1, 2, 4 and 8 on 4 threads have the scores, methods, proof kinds, ceilings,
    assumptions, repeats and previous lines of the committed chain (a game at its ceiling is `start` or `search` in a
    window, by whether its search took a step), and pass `verify_files`; with one thread any window gives the lines of a
    window of one, but for the times and the build;
  * a repeat of a game still in flight (301 of 300) names it, on a later line;
  * a range stopped by a node limit ends at its first unfinished game, with the lines before it in order, and completes
    when run again;
  * a game stopped by a node limit keeps its checkpoint and its clock, stops again from the clock it left, and
    completes under no limit, leaving no clock, with a line whose `seconds.cpu_all_runs` and `active_all_runs` are no
    less than the clock it went on from (the clock file's unit tests are in `src/solver/clock.rs`);
  * every line has `started` and `finished`, UTC stamps, and `cpu`, `active` and `waiting`, whose active and waiting
    seconds add up to the span between the stamps, with windows of 1, 2 and 4; a game that ran no search used no CPU
    time in workers, and a search that took steps did; and the driver's sorting of a game's time (unit tests in
    `src/solver/window.rs`): a game with a worker is active, a search suspended for older games is waiting, and one
    whose workers are still retiring is active;
  * a score that is not the table's fails the range at that game, with the lines before it, as a window of one does;
  * the near-repeat cut of a search that runs when the game before it is concluded (unit tests in
    `src/solver/window.rs`): game 1083, held at no worker beside 1082, is armed by the conclusion of 1082, picks the cut
    up at its first unit, cuts, and ends with the table's score, in fewer steps than without the tactic, which arms
    nothing; the cut is armed once, by a game that assumes nothing, never by one assumed optimal;
  * the policy: claims served oldest first, a lone ready search takes every free thread, a busy one doubles, idle
    workers go back only to another claim and after the last grant has settled, the oldest search reclaims from the
    youngest first and suspends them if need be, and a suspended search claims one thread.
* **The finder** (`tests/solver.rs`): one started after a step changes no score; a game stopped by a node limit and
  resumed with one starts it at once, whatever the clock, and records it in its line.
* **The gate** (`tests/solver.rs`, `tests/soundness.rs`, `tests/cli.rs` and the unit tests of
  `src/search/branching.rs`): the run's `--gate` reaches the searchers of game 780, which takes the steps of conflict
  branching off under a gate wider than every gap and other steps under a gate of 0, and ends with the score of the
  table either way; three gates (0, 100 and 4,000) give each of the small games one answer and some of them other
  trees; a new gate raises the generation and is read back.  The gate trial is off by default, so a gate stays
  unless a test asks for a trial; `tests/cli.rs` checks that a line records no trial by default and with `none`, the
  gates of `--gate-trial GATES`, and the trial of 1000, 800 and 600 that a plan alone asks for, and refuses a plan
  with `none`.
* **The cloud scripts** have tests of their own that need no cloud: the scripts that run the chain unattended
  (`solver/tools/cloud/test/run.sh`: the date and the ledger through `chain.sh`, the lock of `run.sh`, and `status.sh`
  on a chain on another machine, against stand-ins for the aws CLI and ssh), and the build cache
  and the release suite that runs on EC2 before binaries are stored (`solver/tools/cloud/test/test_build_cache.py`,
  `test_run_tests.py` and `test_setup.py`, run with `python3 -I`; [runbook/build-cache.md](../runbook/build-cache.md)).
* **Integrity of results** (`tests/integrity.rs`, `tests/results.rs`, `tests/solver.rs`): previous moves that are not
  what it says is refused; an assumption travels down a chain; a score that is not the table's is a hard error; a
  tampered results file, or one whose line contradicts itself or is of another schema, is refused by both verifiers,
  naming the line; a torn last line is dropped; the committed results pass the verifier and the built-in table holds
  their games, and the proven games past the chain (`results/beyond-the-chain.jsonl`) pass both verifiers with none
  of them in the table; a game stopped by a node limit and resumed reads back assuming nothing; a game's line has the
  CPU time of its workers (`seconds.cpu`, zero for a game that runs no search) and its active time, and its summary line
  ends with those over all its runs and its best promotion sum.  Both verifiers refuse forged repeats (another game
  named; the digests, promotions or promotion sum changed; the game repeated unfinished, after the repeat, or settling
  no search; its assumptions dropped), and `verify-results` refuses two more that only setting the problems up again can
  catch (the Python referee accepts them, as its docstring says).  A search that took no step is the method `start`:
  `verify-results` reads a `search` line with no node, which the Python referee refuses, and a `start` line that took a
  node or did not finish.  The line of a search armed with the near-repeat cut counts the nodes it cut
  (`stats.near_repeat_cut`: game 256 after 255 cuts one, game 21 after 20 is armed and cuts none, and the first game of
  a run and a run without the tactic have no such key), both verifiers accept the files and count the shortened search,
  and both refuse the key on a prime game or with a value that is no count.
* **Incomplete lines and the record of the game before** (`tests/results.rs`): both verifiers accept a file that ends
  with an incomplete line and refuse one whose moves do not score its `best_so_far`; a line's record of the game before
  it (score, moves, proof, assumptions) must be the file's own line of that game, and a line that records an assumption
  the rules do not give is refused.
* **Repeats** (`tests/same_search.rs`): the games that repeat an earlier search are found from the setups alone, and a
  chain concludes exactly those, with the scores of a chain that searches them, from n = 1 (1..300) and from a results
  file that a stopped run left; a game with a checkpoint is resumed, not repeated.  A unit test of `board` checks that
  the problem set up again from n is the one the setup posed, and the same at two set widths.
* **Large games** (`tests/wide.rs`): the Java stopped at 1000, and the sets that hold a game are 8 to 160 words wide,
  so these tests check the widths past 1023 against each other and against the committed results.
  * The same game at every width that holds it (128 and 160 included) has the same frames, candidates, greedy set,
    `max_total`, answer and tree.
  * Game 1133 (W = 20, 41 nodes by default and a few thousand at most) under every tactic against the committed chain, and a
    slice of that chain; the prover's proof of game 1032, whose W = 20 search the tests also resume
    (`solver/noframe/data/exact_parity_2_2531.jsonl`), gives the committed score.
  * A game of 2050 built at W = 64 (it dispatches to 36), one of 4100 at 128 and one of 8200 at 160 go through the
    frames, the matching bound, conflicts, the greedy set and a node-limited search; games of 2050, 2400, 2800 and 3100
    at the widths they dispatch to (36, 40, 48 and 56) and at 64 (2050 also at 128) have the same start and the same
    node-limited search, and 4095 at 64 and 128 the same start, and 8191 at 128 and 160.
  * The games past 4095 (4096, 4100, 4740, 6000, 6392, 8191) at 128 words, and past 8191 (8192, 8440, 9690, 10239) at
    160, are checked against the board layer's own invariants: nine frames, or ten past 8191, the plain game, the
    freedoms, the greedy set and subsets of it promoted and played to the score their sum says, and the root's GF(2)
    bound between the greedy sum and the matching bound, with three to seven block words.  The GF(2) bound's copies of
    four and five words at 128, and of six and seven at 160 (the only width that has them), search game 1000's tree as
    the copy of one word does.  The boards at the steps of the frame count, and the largest, fit in a frame set.
  * The levels of a real depth-first stack of game 1215 (`testdata/n1215_deep_units.txt`): the matching bound from
    scratch and with a member removed against the min-cost-flow reference; and promoting one number at a time
    (with its no-peel shortcut) against the peel of every frame, in reached states of games at W = 20, 28 and 64.  The
    unit tests check the nogood table's lookups against their definition at compact strides of 1 to 16 words, and debug
    builds check that every root frame is winnable, which the shortcut needs.
* **Command line** (`tests/cli.rs`): every subcommand's happy path and its usage errors, through the real binary.
* **Documentation** (`tests/docs.rs`): what the docs state that the code can check, from the table of tactics to
  the links, the citations of headings, the sizes of [WRITING.md](../../../docs/WRITING.md)'s rule 8 and the words the
  project has retired.  The comment at the top of the file lists every check.

## The theory's results

Each proved result of [the theory](../../../docs/THEORY.md) has the checks that bear on it: the suite's own, described
above, and those run outside the suite, by their program.  They test that the programs compute what the theory defines;
the computations behind the statements that are not proved are in [evidence.md](../../../docs/theory/evidence.md).

| results | checks |
|---|---|
| P1 to P4, P10, P11 | `src/playable.rs`, the frame solver's oracle, equals the rules on every subset of the games to 20 (the frame model's exactness, above).  The heuristics' check of the next row finds playability by the rules alone equal to peel's verdict on every set of moves of the boards to 22, and every playable set independent with its own coordinates; no number there has three distinct primes, so it checks P11's equivalence on every set.  The heuristics' Solvent makes the same moves with maximal factors as with every proper divisor on every game to 1000 (`heuristics/core.py`). |
| C1 to C3 | `heuristics/evaluation/fm_relation.py`, run in heuristics/ as `../.venv/bin/python -m evaluation.fm_relation`, 85 s on one thread.  On all 2,097,152 sets of moves from {2, …, 22}, playability by the rules alone, Franklín and Moniot's condition, per = 1 for some T and peel agree, and so do independence of the family of P10, independence at both parities and an odd per; 1,112 sets have a payment scheme and are dependent, the smallest {6, 10, 15}, and none is an odd core.  The heaviest sets of the three kinds give opt(n), the parity bound and the Franklín–Moniot bound, the prover's values on every board to 22.  On 18,200 random sets of the boards 30 to 120, grown around members of three or more primes, the same holds, with 500 odd cores and 570 dependent sets with a scheme.  Board 44 has no odd core of at most six members, and board 45 one, {12, 18, 20, 30, 45}. |
| N1, N6 | The prover's enumeration of optimal sets (`solver/noframe/src/bin/optsets.rs`): the sets derived at the ceiling and at the primes equal those enumerated directly on every such game to 700, and to 260 in its tests; every optimal set found holds q(n) and no other prime. |
| J3 | The prover's `solver/noframe/src/semiprime.rs`.  On every odd n to 300 with two prime factors, the values of opt(n) found by the prover's search from nothing (the search itself is checked against brute force over all sets to 20) satisfy opt(n) − opt(n − 1) = n − x exactly where n closes a cycle with U\*_2(n − 1); x taken as the largest member of the cycle instead of the smallest breaks the formula on every such game.  Every such n to 1300 that closes a cycle meets the formula in the committed chain, which `relation` checks (it stops, naming n, at a score off it), and the optimal sets derived by the exchange of x for n are the enumerated ones on every such game to 260. |
| N3 to N5, R6 | The chain bounds `score(n − 1) ≤ score(n) ≤ n + score(n − 1)` and the prime formula, which both verifiers check on every line of the committed chain, and the repeats (`tests/same_search.rs`; `verify-results` sets both problems of every repeat up again). |
| L2, L4 | `tests/below_the_ceiling.rs`: at every game n = 8p of the committed chain that L2 covers, and every n = 12p that meets L4's hypotheses with its recorded optimal set, opt(n) is the theorem's bound, and the recorded optimal set of game n − 1 with 4q(p − 1) or 6q(p − 1) exchanged for n, ordered by the peel, plays by the rules to it (the later games' exchanges in long tests).  The bound fails at 184, which L2 does not cover, and at 204, which fails only L4's 15q(p − 1) > n. |
| L3 | `tests/below_the_ceiling.rs`: its example, the recorded optimal set of game 371 with 172 exchanged for 372, is not playable. |
| R4 | The test of `src/search/node_bound.rs`: the bound of the root's node is the root's GF(2) bound and not below P\*.  The rest is the exactness of the searches, which equal the Java's optima and the brute force (above). |
| R5 | No check: no code starts a search from a floor. |
| Q1 | A search over the states of the pot that uses no theory (`heuristics/evaluation/certify.py`) finds the heaviest set of sources unique in every game to 62. |
| Q3 | The prover's `src/moves.rs`: ℓ(n) and the heaviest longest game against brute force to 22, and ν(n) against the matching solver on every board to 400. |
| W1 | The heuristics' check above, to 22.  The prover's parity bound is at least opt(n) on every game to 2531 (`solver/noframe/data/exact_parity_2_2531.jsonl`), and the recorded moves are independent at both parities on every game to 2531 (`exact` asserts both before any search); the intersection agrees with brute force on 3,000 random pairs of matroids ([matroid intersection bounds](https://github.com/bvchess/taxman-notebook/blob/main/search/matroid-intersection-bounds-2026-10/README.md)). |
| W4, W5 | `solver/noframe/tests/parity_steps.rs`: on the prover's committed parity bounds of every board from 2 on, the greedy basis carried from board to board (and compared with a fresh greedy at sampled boards) gives each step of W4, and the bound rises as W4 says, by n − x_G at every exchange step, where x_G is the shadow of n; on the committed chain, opt(n) rises as the bound does at every exchange step and half-out step, among them every prime and every odd semiprime from 189. |
| F1, F3, F6 | The censuses of every board 2..8191 and 8192..10239 (above): their check A, that each frame lies in one layer and the frames in front of it one layer down, and their check C, clean numbering; the unit tests of `frames` (above). |
| F4 | `tests/bottom_layers.rs` (above). |
| X1 to X7 | `tests/model_exact.rs` (above), and the censuses' checks D to G: no candidate has a back claim's factor among its maximal factors (X5), none is a factor of a frame numbered 0 and every one has a home (X3, X6), and the sources played are the U\* of `src/root_problem.rs` (X1); check I and `tests/canonical_root.rs` (above), that setup builds Π_n. |
| E5 | The first census's check H (above): Π⁺(∅) with the claims is Π_n, on every board to 8191. |
| B1, B4 | The study of the multi-field bound ([its entry](https://github.com/bvchess/taxman-notebook/blob/main/search/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md); its programs build at commit 8753189): every promotion set of the boards with at most 22 candidates, and sampled ones of the boards 137..400, independent under the plain signing and five random signings over GF(3) of every frame, 6.4 million pairs, and 14,472 of every 25th board of 400..2175 in 4,320 signed frames; the decomposition bound never below P\* under random multipliers at 309 roots of 2..400 and at 400..2175. |
| B2 | The GF(2) bound under bound soundness (above).  At every root of 2..1300 that is not prime the bound is at least P\*, and every promotion set of the 90 roots with at most 22 candidates is independent in every frame ([matroid intersection bounds](https://github.com/bvchess/taxman-notebook/blob/main/search/matroid-intersection-bounds-2026-10/README.md)). |
| R2, R3, R7 | The prime exchanges and the near-repeat cut under bound soundness, the soundness harness, and the tripwires (above). |
