# The tests

Part of the [design](../DESIGN.md).

What the full suite checks, by subject.  A few tests sample what an exhaustive run would cover (every 18th golden game
for the peeling agreement, every sixth game above 511 for the root promotability of the frame builder's candidates,
every second line of the Java fixtures), and the comment on each says what the whole run costs.

* **Unit tests**, per module, include the Java's unit tests' cases (of the factor table, frames, and frame sets, for
  n = 85).  The helpers that the unit and integration tests share (temporary directories, a random generator) are one
  file, `tests/common/support.rs`.
* **Golden tests** (`tests/golden_frames.rs`, and unit tests in `src/search/tests.rs` and `src/bounds.rs`) check every
  stage of the port against the Java's
  answers in `testdata/golden/n*.json` ([its README](../../frame/testdata/golden/README.md) says where they came from):
  the setup's frames, the numbered and merged frames, the candidates, the reduced frame set, the greedy incumbent, the
  prime shortcut and the result of the solve.  The searched golden games (94, up to 760) give the Java's promotion sums
  with the default tactics, and the tactic variants give the same answers on those up to 400.  Past 760 the Java's
  structure is fixed in `testdata/golden_wide/` (n = 511, 512, 1000, 1001, 1023, 1024, 1025, 1100, 1231, 2047 and 2048,
  checked at the width `with_width!` picks) and `games_java.txt` (the Java's game of the board's frames, move for move,
  for 171 sets of promotions at 511–2049).
* **The board layer against the chain** (`tests/board_layer.rs`): for every 25th searched game of
  `results/strict.jsonl`, and the last, the moves that the game's promotions make and the digest of the board's search
  problem are the line's, which the golden data (up to 2049) cannot say of the games that matter.  The unit tests of
  `frames` check what the frames remember (which numbers each can take on, which sources `removeImpossible` has been
  through) against a fresh computation.
* **The frame model's exactness** (THEORY.md, Theorems B and M; [frame-model.md](frame-model.md#what-optimal-means)):
  * `tests/model_exact.rs` decides playability on the whole board with an oracle that shares no code with `frames` or
    `model` (`src/playable.rs`, the peel, itself checked against the rules on every subset of the games to 20).  It
    checks both directions of Theorem M on every set of every game to 100 (to 120 ignored) and on random sets of games
    to 1300 (to 8191 ignored), and the near miss that only the candidate filter keeps out (n = 98).
  * `tests/model_extended.rs` checks Theorem M⁺ (the extended problem, with the sources of three or more prime factors
    optional) both ways, in both formulations (with and without the claims of `unzip`, Lemma M5⁺): on every set of
    every game to 80 (to 60 with every such source open) and on random sets to 1300 (to 4095 ignored).  It checks that
    with none open the promotion sets are the frame model's, and catches a mutant that skips the check of Lemma M5⁺.
  * `tests/bottom_layers.rs` checks Theorem B on every game of `optimal.json`, the committed chain and another run of
    its games 2176–2276 (`testdata/c8g.chain-2176.jsonl`), every optimal set the prover enumerated, and every optimal
    set of the games to 48.
  * The unit tests of `frames` check that `unzip`'s claims are its mandatory pairs, that `numbering` is what
    `build_frames` uses, that the frames lie in layers (Lemma Ω, also on punctured boards), and that the numbering by
    layer is the Java's breadth-first numbering on every game to 400 and a sample to 8191.
  * The census of every game (`examples/structure.rs`; its data are in
    `solver/notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/`) checks what Theorem M's proof says of setup
    one game at a time, and that Π⁺(∅) with the claims of `unzip` is Π_n (check H).
* **Regression**: `taxman regress --to N` solves 1..N as one range and checks every score against `optimal.json`.
* **Tactic invariance**: games 1..300, forced to search, give identical scores with each tactic off or changed, and a
  checkpoint is refused when any of the search tactics differs.
* **Bound soundness**:
  * on small frame sets, the upper bound is never below the best completion computed exhaustively; the same for the
    matching bound and its refinement by conflicts (whose exact completions must be promotable, and whose conflicts,
    carried down a path, must stay conflicts), and for the Lagrangian bound over the root's nogoods with fitted and
    random multipliers;
  * the incremental solvers agree with the reference min-cost flow (with the Lagrangian weights, live nogoods and
    constant worked out afresh, under random relaxations changed mid-walk) and with a solver started afresh, and their
    levels pass the optimality certificate;
  * the GF(2) bound (`src/gf2/tests.rs`, `src/gf2/meet.rs`): the intersection against brute force under every size
    limit, from every set it reports as a warm start, with members deleted, and its weight split's bounds without each
    member.  On the games up to 400 the bound is never below the exhaustive best completion, and the optimal
    promotions are independent in both matroids.  Along random paths of games 60–1000 (promotions, deletions,
    backtracking) every level equals the matroids built afresh at its node, every member it calls forced is (the bound
    without it, solved afresh, is below what the node needs), and every tableau derived instead of built is the one a
    solve builds; along the same kind of paths the copies of every word count (1 to 5, padded) give the same results
    and do the same work.  Blocks of more than 255 members (five words) are checked against the greedy algorithm, one
    matroid free and the other random.  At the 600 nodes of the probe's fixtures (936 and 1215), completions promoted
    in random orders are independent in the node's matroids and no heavier than its bound, and the matching path's
    search started from the bound finds nothing above it (`tests/soundness.rs`);
  * conflicts and residuals are checked against promoting the sets;
  * the prime exchanges (`src/search/symmetry.rs`): the symmetric pairs against their definition (every number looked
    at, every pair of primes tried) on the games 2..=300 and a sample to 2200; the images of the optimal set and of
    random maximal promotable sets promotable and worth the formula; every optimal set keeping every inequality; on the
    games with at most 22 candidates, every promotable set so checked, and the best that keeps the inequalities worth
    P\*; and the rule's cuts and drops against its definition by enumeration (exact where the pairs are independent).
* **Soundness harness** (`tests/soundness.rs`): the search starts from an incumbent worth 1 or 5 less than P\* with an
  empty payload, so that any pruning that loses every best completion leaves the fake incumbent in place (which tests
  the `<` and `<=` boundaries too).  It runs:
  * every game 2..=300 under each of 41 configurations: 19 of the matching path (every tactic; forgetting every step,
    every third, never; the Lagrangian bound refitted every few steps; one thread and two), 11 of the default, the
    GF(2) bound with the rule of symmetric pairs (every way of choosing the branch; the matching bound as a first check;
    without forced members or nogoods; forgetting every step; two threads; the Lagrangian bound after it), the GF(2)
    bound without the rule on one thread, two, and forgetting every step, the matching path with the rule, and 7
    policies of keeping nogoods;
  * every game 2..=600 with symmetric pairs, with the rule over either bound (and the oracle without the frame model to
    40); units on the way to an optimal set (an optimal set less 30 to 70% of its members, games 600..1000) with the
    rule; and games 301..=600 (every tenth: searches of hundreds to thousands of steps) with the Lagrangian bound
    refitted early and often, over either bound, where it cuts and forces thousands of times;
  * the same for 500 perturbed frame sets (numbers dropped, a role flipped, moved a frame) with the truth found
    exhaustively; promoting a set at once against one by one in random orders; conflicts and nogoods on perturbed
    models;
  * searches interrupted every 1,000 nodes and resumed from a checkpoint with units spilling: game 600 on the matching
    path, one of them with the Lagrangian bound refitted every four steps, and game 992 with the GF(2) bound, whose
    search of 600 is too short to stop;
  * units of real games at W = 20 to 64 (thinned so that the truth is found exhaustively) from a handicapped incumbent,
    and games 1133 (matching) and 992 (GF(2)) resumed every 1,024 steps at W = 20, 24 and 64, which carry the harness
    past the widths of the Java's table (n ≤ 1000).

  A search below the root with the rule of symmetric pairs is held to the best completion that keeps their
  inequalities (an exhaustive oracle of its own, which prunes by the same bound but shares no code with the rule), which
  it must reach; at the root that is P\* (Theorem 13).
* **The floor** (`src/search/tests.rs`): on every game of the Java's that needs a search, to 400, a floor below P\*
  finds an optimal set, a floor at P\* or above completes with nothing found, and a floor at the root's bound or above
  ends at the first check; with the default tactics and (every ninth game) on four threads, on the matching path, and
  without the rule of symmetric pairs.  The floor at P\* searches the tree of the search seeded with an optimal set,
  step for step, on games 300 to 1000; a floor at the ceiling ends the search before it starts; a stopped floor search
  resumes under its floor, and is refused under another floor or none.  With no floor nothing moves: the tripwires and
  the probe's fingerprints are the same.
* **Node-count tripwires** (`tests/tripwires.rs`): a pruning tactic that stops working changes no answer, so no
  assertion on answers sees it (a mutant that ignored the result of `Nogoods::ruled_out` takes 27 to 67% more nodes).
  With one thread the tree is deterministic, so the test asserts the exact node count of a fixed game under fixed
  tactics: n = 480 to 1133, every tactic off or changed in turn, with the GF(2) bound and on the matching path, and the
  tree of game 600 on the matching path with the searchers forgetting their nogoods every 500 steps.  The matching
  path's rows are the default's trees before the GF(2) bound, its rows with `lagrange=off` those of the build before
  that tactic, and the rows `symmetry=off` the default's before the rule of symmetric pairs.  The table `EXTENDED` does
  the same for the extended problem with nothing open, Π⁺(∅), without the claims of `unzip`, whose search must also be
  the default engine's on the same problem, node for node; with the claims, Π⁺(∅) must pose the default problem itself
  and reproduce every row of the default table, node for node.

  **A change that alters the tree on purpose fails them.**  Run `cargo test --release --test tripwires -- --nocapture`
  (the quick tests check only the rows of trees of 250 nodes or fewer): it prints the rows as observed, in the layout of
  the table at the top of the file.  Check that the promotion sums (the answers) have not moved, paste the rows in, and
  update any node counts quoted in the docs.
* **Extended mode** ([Extended mode: one-sided candidates](search.md#extended-mode-one-sided-candidates)):
  * the model with one-sided candidates against the extended problem's reference predicate on every game 2..=300, for
    nothing open, every source of three or more prime factors open (to 120) and five random sets of them
    (`tests/model_extended.rs`: sets promoted at once and one by one in random orders, their conflicts and nogoods, the
    narrowing and the count);
  * its search against the whole-board oracle on every game to 80 (to 60 with every such source open, 69 ignored);
  * the GF(2) bound's free blocks: the intersection against brute force with coloops, every promotion set of the games
    to 120 independent and under the bound, and random paths to 1300 matching the matroids built afresh at every node;
  * the soundness harness in extended mode on every game 2..=300 under eight configurations, and a game resumed every
    1,000 nodes;
  * the header's digest of the one-sided candidates, and the refusals of what extended mode cannot take (tactics, block
    words, another problem's checkpoint).
* **Certificates** (`tests/certify.rs`):
  * `taxman certify`: every game of 600..=1300 that needs a certificate certified (ignored; a sample by default), and
    some past 1300; a planted wrong score beaten by a game the referee accepts, with the status 2 and the
    counterexample file, and said again by a rerun; a certificate's search stopped every 1,000 nodes and resumed to the
    same verdict; the games a certificate cannot be made for refused; the file of certificates and the bounds' files
    read as they should be; `--list` certifying the games of a file in its order;
  * `taxman verify-certificates`: a file that `certify` wrote over 1312..=1317 (three games that rest on the game before
    them) passes, on one thread and on two, with the summary's counts and the committed prover files' proven end of the
    chain.  Each doctored line (another digest, score, open members or width; a `same_problem` with nothing to rest on,
    or with its numbers changed; a certified line with a counterexample; a `build` without a commit; a score the
    results do not have) is refused, naming the line.  A beaten line is verified with its counterexample, and cannot
    stand beside a certified one.  The end of the chain is checked on synthetic data (a gap stops what rests on it) and
    on the committed chain;
  * the two formulations: a line records its formulation, and an old line reads as without the claims.  The rule
    settles the same pairs in each formulation and across them (a certificate of one settles a later game in the
    other, with the digest of the problem in the run's formulation, and as a search of the game does).  The claims
    change the problems of 231 of the 377 games of 600..=1300 that need a certificate, and none of their sizes, and make
    the rule settle 101 games where it settled 81 (609 and 611, say, pose different problems without the claims and one
    with them).  A file that mixes the formulations passes, and a line whose formulation is changed is refused.
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
    refused; a checkpoint of another game, with other tactics, or with an incumbent that does not fit is refused; and
    the binary rejects bad times, survives a closed stdout and stderr, and stops on SIGINT and SIGTERM with a checkpoint
    it then resumes;
  * the control: a ceiling given before a run, or while it runs at the incumbent it has, ends it as optimal (with no
    search work if the start reaches it); offers from outside raise the incumbent; a stop through it checkpoints a run
    that resumes exactly; workers added, retired and down to none (suspended, then resumed, checkpointed, stopped, or
    ended by a ceiling) give the exact answer; and the conservation test runs with a controller that sets 0 to 4
    workers every millisecond or so while the run checkpoints, spills and resumes;
  * the counters: capacity is wall × threads when the workers do not change, and a searcher that sleeps before its
    first and second polls shows in the rebuild and first-step times.
* **The window** (`tests/window.rs`; the policy's unit tests in `src/solver/window/allot.rs`):
  * chains 1..600 with windows of 1, 2, 4 and 8 on 4 threads have the scores, methods, proofs, ceilings, assumptions,
    repeats and previous games of the committed chain (a game at its ceiling is `start` or `search` in a window, by
    whether its search took a step), and pass `verify_files`; with one thread any window gives the lines of a window
    of one, but for the times and the build;
  * a repeat of a game still in flight (301 of 300) names it, on a later line;
  * a range stopped by a node limit ends at its first unfinished game, with the lines before it in order, and completes
    when run again;
  * a game that passes the wall (`game_limit`) ends the range with no line for it, its checkpoint and clock kept, walls
    again at once under the same limit from the clock it left, and completes under none, leaving no clock (the command
    through the binary is in `tests/cli.rs`, the clock file's unit tests in `src/solver/clock.rs`);
  * a score that is not the table's fails the range at that game, with the lines before it, as a window of one does;
  * the policy: claims served oldest first, a lone ready search takes every free thread, a busy one doubles, idle
    workers go back only to another claim and after the last grant has settled, the oldest search reclaims from the
    youngest first and suspends them if need be, and a suspended search claims one thread.
* **The cloud scripts** have tests of their own that need no cloud: the scripts that run the chain unattended
  (`solver/tools/cloud/test/run.sh`: the wall and the ledger through `chain.sh`, `wall-node.sh` through each way a run
  ends, `wall.sh` through a launch, a stop and a clean-up, against a stand-in for the aws CLI; [solver/docs/runbook/wall.md, "What
  was tested, and what can go wrong"](../runbook/wall.md#what-was-tested-and-what-can-go-wrong)), and the build cache
  and the release suite that runs on EC2 before binaries are stored (`solver/tools/cloud/test/test_build_cache.py`,
  `test_run_tests.py` and `test_setup.py`, run with `python3 -I`; [runbook/build-cache.md](../runbook/build-cache.md)).
* **Integrity of results** (`tests/integrity.rs`, `tests/results.rs`, `tests/solver.rs`): a previous game that is not
  what it says is refused; an assumption travels down a chain; a score that is not the table's is a hard error; a
  tampered results file, or one whose line contradicts itself or is of another schema, is refused by both verifiers,
  naming the line; a torn last line is dropped; the committed results pass the verifier with the counts that
  `results/README.md` quotes; a game stopped by a node limit and resumed reads back assuming nothing.  Both verifiers
  refuse forged repeats (another game named; the digests, promotions or promotion sum changed; the game repeated
  unfinished, after the repeat, or settling no search; its assumptions dropped), and `verify-results` refuses two more
  that only setting the problems up again can catch (the Python referee accepts them, as its docstring says).  A search
  that took no step is the method `start`: `verify-results` still reads the `search` line of an older build, and the
  Python referee refuses it, and a `start` line that took a node or did not finish.
* **Keeping going past limits** (`tests/keep_going.rs`, `tests/results.rs`, `tests/sweep.rs`,
  `solver/tools/test_sweep_plan.py`, `solver/tools/cloud/test/run.sh sweep`):
  * a range whose games all stop at a limit goes on, each game built on the best game of the one before it, with no
    ceiling; a prime game after an unfinished one is searched, and after a proven one derived;
  * a rerun passes over the unfinished games; `--retry` and `--retry-improved` search them again (the latter only where
    the game before has gained, the rule also in a unit test of `window`), and the lines name the pass;
  * both verifiers accept a file with holes and unfinished lines, and refuse a ceiling, a `cap` or `prime_formula`
    proof, a changed score, moves or assumptions, or a later line, for a game built on one;
  * the sweep's planner tiles a range into blocks of equal work and cuts the excerpts of the passes, and `sweep.sh` over
    1301..1340 and 1411..1416 completes with both referees accepting the file, is stopped by a SIGTERM and resumed, and
    settles a block's first game by the ceiling its neighbour gives in pass 2.
* **Repeats** (`tests/same_search.rs`): the games that repeat an earlier search are found from the setups alone, and a
  chain concludes exactly those, with the scores of a chain that searches them, from n = 1 (1..300) and from a results
  file that a stopped run left; a game with a checkpoint is resumed, not repeated; a repeat assumes what the game it
  repeats assumes.  A unit test of `board` checks that the problem set up again from n is the one the setup posed, and
  the same at two set widths.
* **Known games** (`tests/known.rs`, with hand-made sources: a store of games 906..944 proven with the truth's games, a
  committed chain of games 1..60, and the table):
  * a chain over 900..960 records the store's proven games with no search, citing their lines and assuming U at the
    games they rest on, and the others with the scores of a chain without the store (and fewer nodes in all); a game
    carried from the game of another line is played and recorded;
  * the committed chain's and the table's games are recorded with their own assumptions, and the game before the range
    from each of the three has the source in `prev`;
  * an open game's upper bound ends its search (proof `known`, citing the one line), and its lower bound's game seeds
    it (fewer nodes); `force_search` leaves the games to their searches; a store game that does not play, or that the
    chain bounds contradict, stops the run at its game;
  * both verifiers accept those files, and refuse each of: an upper bound of the store worth another value, a missing
    lower or upper line, a line by another rule or from another run, no store given, a citation of other lines,
    assumptions dropped or added, no citation, a changed proof or score, a citation on a game that is not a known one.
    `tests/docs.rs` has `known` among the keys a line has.
* **Large games** (`tests/wide.rs`): the Java stopped at 1000, and the sets that hold a game are 8 to 128 words wide,
  so these tests check the widths past 1023 against each other and against the committed results.
  * The same game at every width that holds it (128 included) has the same frames, candidates, greedy set,
    `max_total`, answer and tree.
  * Game 1133 (W = 20, 41 nodes by default and a few thousand at most) under every tactic against `strict.jsonl`, and a
    slice of that chain; the
    sacrifice bound proves game 1133 by an independent route.
  * A game of 2050 built at W = 64 (it dispatches to 36) and one of 4100 at 128 go through the frames, the matching
    bound, conflicts, the greedy set and a node-limited search; games of 2050, 2400, 2800 and 3100 at the widths they
    dispatch to (36, 40, 48 and 56) and at 64 (2050 also at 128) have the same start and the same node-limited search,
    and 4095 at 64 and 128 the same start.
  * The games past 4095 (4096, 4100, 4740, 6000, 6392, 8191) at 128 words are checked against the board layer's own
    invariants: nine frames, the plain game, the freedoms, the greedy set and subsets of it promoted and played to the
    score their sum says, and the root's GF(2) bound between the greedy sum and the matching bound, with three, four and
    five block words.  The GF(2) bound's copies of four and five words (only at 128) search game 1000's tree as the
    copy of one word does.
  * A committed checkpoint of game 1133 (`testdata/n1133_after_1000_nodes.ckpt`, W = 20) must still resume: a
    deliberate change of the checkpoint format fails it, and the test says how to make the file again.
  * The levels of a real depth-first stack of game 1215 (`testdata/n1215_deep_units.txt`): the incremental matching
    bound and the quick bounds without a member against the min-cost-flow reference; and promoting one number at a time
    (with its no-peel shortcut) against the peel of every frame, in reached states of games at W = 20, 28 and 64.  The
    unit tests check the nogood table's lookups against their definition at compact strides of 1 to 16 words, and debug
    builds check that every root frame is winnable, which the shortcut needs.
* **Command line** (`tests/cli.rs`): every subcommand's happy path and its usage errors, through the real binary;
  `bench --history FILE` keeps the tracked `solver/frame/bench/history.jsonl` out of the tests.
* **Documentation** (`tests/docs.rs`): what the docs state that the code can check, from the table of tactics to
  the links, the citations of headings, the sizes of [WRITING.md](../../../docs/WRITING.md)'s rule 8 and the words the
  project has retired.  The comment at the top of the file lists every check.
