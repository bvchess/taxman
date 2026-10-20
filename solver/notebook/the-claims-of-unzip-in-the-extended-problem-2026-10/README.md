# The claims of `unzip` in the extended problem (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-15.  Concluded 2026-10-16.

**Question.**  The extended problem Π⁺(O) claims nothing of what `unzip` claims in Π_n.  That costs time (+11% on Π⁺(∅))
and keeps repeats apart: 3427 has 3425's search problem but another Π⁺, differing only by the pair (3426, 1713) that
`unzip` claims in Π_n, and 115 repeats of the work list (361 thread-hours, 3427 alone 223) are deferred for that reason.
Can Π⁺(O) keep the claims where they are still moves of every instance, so that Π⁺(∅) is Π_n and more repeats rest on
the game they repeat?  **Answer.**  Yes, by a rule that keeps a frame's claims when it holds no member of O ∪ R (Lemma
M5⁺): no mismatch on 5.9 million sets against the oracle, and the same-problem rule settles 101 games of 600..1300
against 81 and one more repeat past 1300, but none of the 115 deferred repeats.  A rule that keeps claims one by one
settles 465 of 1,575 games against 361 and 340 and 60 of the 115, and was measured, not kept.

## Method

**The rule** (8c833a1).  `build_extended(t, open, claims)` takes the frame model's claims (`SearchFrames::claims`, in
the order made) and gives each the frame, as setup forms it, of its source.  A frame keeps its claims if it holds no
member of O ∪ R and the factor f of each of its back claims passes the check (`frames::ClaimCheck`): f is no two-sided
candidate of Π⁺(O), and no candidate has f among its maximal factors.  Its pairs then leave the moves and the factors of
its frame of Π⁺(O) before the root is reduced.  The candidates do not change (a claimed factor is none: debug-asserted),
nor do the freedoms or `in_the_bag`⁺.  `ExtendedFrames` records the claims kept and those refused.  Lemma M5⁺ ([the
extended problem](../../../docs/theory/extended-problem.md)) says that the claims kept are front and back moves of every
instance of Π⁺(O), so Theorem M⁺ holds with them out; by its corollary the two formulations have the same promotion
sets, so a certificate in one bounds the heaviest promotion set in the other; and Π⁺(∅) with the claims is Π_n.  The
formulation is a parameter of `Board::extended` and of everything that sets Π⁺(O) up.

**Tests** (8b0d0d4, 6146627).  Theorem M⁺ against the whole-board oracle with the claims kept: every set both ways on
every board 2..80 with O empty and three random O (1.66 million sets), and 2..60 with every source open (4.2 million);
random sets at 300, 600, 1000, 1300, 741, 1100 and 1200 (a rejected source in some sets), and, ignored, at 2000, 3000
and 4095; the search's optimum against the oracle on every board to 80 (60 with every source open).  There is no
mismatch.  The mutation that skips the check (the refused pairs taken out anyway) is caught on every problem to 80 where
the check refuses a claim (24, 25 and 26 with all of U\*_{≥3} open but one frame's sources).  Π⁺(∅) with the claims
poses Π_n's problem (frames, candidates, root, greedy set, `max_total`) on every board to 300 in the unit tests and on
every board 2..8191 in the census, and its search reproduces every row of the tripwires' default table node for node.

**The census** (`examples/structure.rs`, check H: Π⁺(∅) with the claims is the frame model's problem), on the code of
631e027 and 6146627, on the M1 Max, `nice -n 10`.  Where Π⁺(U\*_{≥3}) fits in nine frames (to 4095) it records the back
claims at risk (their factor fails the check against the candidates of Π⁺(U\*_{≥3}), the most any O has) and those
refused for some O (for each frame holding one at risk, the largest O free of it).

**The certificates** (3841753).  Lines record `"claims": true|false` (absent: false); `certify --no-claims` makes the
old formulation.  The rule of the same problem compares the two problems whole in the run's formulation, whatever
formulation m's line was made in (by the corollary m's certificate bounds the heaviest promotion set of either): a line
of the other formulation is passed over unless its frames and candidates, which the claims do not change, are n's, and
is otherwise set up again in its own formulation (it must reproduce its line) and then in the run's.
`verify-certificates` sets each line up in its own formulation and checks the rule in the formulation of the line that
rests on the other.

**Claim by claim** (e62dd1f, 30c8504).  The proof needs, for a back claim (s, f), only that no member that can be in an
instance lists f but s, after the earlier pairs, and for a front claim only that s lists f alone after them.  So each
claim can be checked on its own, in the order made: a claim (s, f) with s fixed (not open) is kept if, after the claims
kept before it, (front) every maximal factor of s but f is the factor of a claim kept before it, or (back) f passes the
check and every maximal composite of f other than s that is a source of s's frame or a member of R in it is the source
of a claim kept before it.  The proof is Lemma M5⁺'s, by induction on the claims kept.  With O empty the conditions are
those under which `unzip` claimed, so Π⁺(∅) is still Π_n.  It replaces the frame-wide test of `build_extended` (twenty
lines) and was measured on this code, then taken out.

## Results

**The census.**  Check H holds on every board 2..8191.  The check refuses claims on 10 boards to 500 (24-26 and
343-349), on most from 507 and on every one from 1501, and a growing share of them (at the worst O for each frame): 2.5%
of the back claims of 501..1000, 5.3% of 1501..2000, 9.2% of 3001..3500, 11.3% of 3501..4095.  The extended problem with
every source of three or more prime factors open needs 9 frames on 2048..4095 and 10 on every board from 4096, which a
frame set (9 at most) cannot hold: certificates past 4095 wait for a tenth frame.

**Certificates 600..1300**, one thread, with the claims: the 377 games are certified again, none otherwise.  The rule
settles 101 of them against 81 (the same 81 and 20 more, 611 on 609 to 1147 on 1145), each as a direct search of the
game in the same formulation does.  The 276 searched take exactly the nodes they took without the claims (296,743) and
29.4 s against 30.5 s.  `verify-certificates` passes the file beside the one without the claims, and a file that mixes
the two.

**Past 1300** (every certificate's Π⁺(O) set up with the claims, 78 s): the claims change the problem of 346 of the
1,575 games that need a certificate (231 of 377 in 600..1300, 43 of 490 in 1301..2175, 72 of 708 in 2176..3440, none
past 2688), and the rule finds 361 repeats against 340: one more past 1300 (1371).  None of the 115 deferred repeats is
settled, 3427 neither: 3425 and 3427 keep their problems of before.  At 2112 and 2156 (|O| 69 and 70), certified three
times each way, alternating, on one thread, the problem (digest) is the same with and without the claims, so the trees
(15,119 and 43,599 nodes) and times (3.42 and 7.70 s) are too.

**Why the deferred repeats stay.**  The frame of layer 3 that holds most of a board's sources (joined through the
factors 2p, 3p, 6, 9, 10, 15, ...) holds open sources as soon as some are open, and then every claim of it is refused,
the pair (3426, 1713) among them, although its check passes (1713 has no maximal composite but 3426, and layer 2 is
closed): 3426 = 6 · 571 shares the factor 6 with that frame, which at 3427 holds 1719 = 9 · 191, open.  The condition
"holds no member of O ∪ R" is what the lemma's proof uses to know which members of the instance list a claimed factor;
it does not need the whole frame free.

**Claim by claim: measured, not kept.**  Every default and ignored test of `tests/model_extended.rs` passes with it (the
oracle both ways, the search, the mutation caught).  On the problems of 600..3440 it settles 465 repeats against 340
without claims and 361 with the rule as designed, among them 60 of the 115 deferred, 278 of their 361 thread-hours, and
3427 on 3425 (one problem, digest d0b64c26c38bf1b2).  At 2112 and 2156, three times each way, alternating, the trees are
the same and a certificate takes 3.22 s against 3.53 s (8.7% less) and 7.60 s against 7.97 s (4.6% less).

**Checks.**  No tree of the default mode changed: the probe's fingerprints 7e8e724717d26545 and f3deaac0dbdc030a
(`gf2=on`), the tripwires, `regress --to 1000 --threads 1` (9.2 s), and the one-thread chain 1..1300 line for line
equal, but for the times and the build, to the base commit's binary (ed09ac0; 290,503 nodes), both test suites, clippy
and fmt.  The laptop ran one process at a time, on one thread, `nice`; nothing ran on AWS.

## Decision

* **Kept.**  The frame-wide rule: `certify` searches Π⁺(O) with the claims of `unzip` by default, and Π⁺(∅) is Π_n.
* **Not kept: claim by claim**, a change of the design for the next consultation of the theory track.  It would settle
  60 of the 115 deferred repeats (278 thread-hours) and shorten a certificate by 5 to 9%.  The rule is paused (decided
  2026-10-16, until after the weekly reset), because certifying U is not the priority ([STATUS](../../docs/STATUS.md)).
  Its branch `worktree-agent-a2db9341d6d4e8cd9` held three commits: e62dd1f (the rule, `Claims::Each`, the default of
  `certify`), 30c8504 (the certificates of 600..1300 and the problems of 600..3440 claim by claim) and 54dce27 (the edit
  of THEORY.md, uncommitted until then).  The branch is on the old layout (`rust/`, `heuristics/THEORY.md`), so its
  paths need the new ones before it merges; by the project's rule it is deleted once recorded here.
* **Reopens** when certifying U becomes the priority again.

## Files

The census checks what the module comment of `examples/structure.rs` defines (every field of a line) and what
[THEORY.md](../../../docs/THEORY.md) proves (Lemma Ω, N and Theorem M, whose hypotheses it checks).

| file | command | what it is |
|---|---|---|
| `structure_2_8191.jsonl.gz` | `structure 2 8191 --threads 1 --out structure_2_8191.jsonl`, then `gzip -9`, on the code of 6146627 (`cargo build --release --example structure --features studies`); 1,271 s summed over the boards | one line per board 2..8191, every board the solver supports: the checks A, C, D, E, F, G and H, the frames by layer, the layers of every number, the claims of `unzip`, the back claims whose factor has a composite among the factors, the frames of the extended problem with every source of layer 3 or more open (`extended_frames`), and the back claims at risk of the lemma's check and those some set of open sources refuses (`m5_at_risk`, `m5_refused`) |
| `summary.txt` | the summary the same run printed | `PASS`: every check holds on every board; frame 0 is layer 3 on 8,100 boards, layer 2 on 80 (all at most 186: exactly the boards where a frame of layer 2 has freedoms), and none on the 10 boards with no frame with freedoms; on every board the numbers follow the layers; `Home::MergedAway` is reached on one board (114), twice, and never counts |
| `certificates_claims_600_1300.jsonl` | `taxman certify 600-1300 --results results/strict.jsonl --upper U --out certificates_claims_600_1300.jsonl --threads 1 --progress 0` (code of cd959d1; 29 s of search) | the 377 games, every one `certified` again, each line with `"claims": true`: 276 by a search (296,743 nodes) and 101 by the same problem; `verify-certificates` passes it beside the file without the claims |
| `problems_claims_600_3440.jsonl` | `extended problems 600 3440 U --known results/strict.jsonl --out problems_claims_600_3440.jsonl` (code of cd959d1; 78 s) | the problems of [the certify tools](../../tools/certify/README.md) (`problems_600_3440.jsonl`) again in the formulation with the claims (each line with `"claims": true`): 361 games with a `same_as` against 340 |

`U` stands for the four `--upper` files of the certify tools, `solver/noframe/data/upper_*.jsonl`.  The census earlier
ran on the code of 631e027 (the claims of `unzip` recorded as front and back pairs) in 156 s on four threads, 624 s
summed over the boards (0.27 s at most, n = 8129), and its lines are reproduced field for field but for `secs` by the
later runs (layer numbering, `extended_frames`, the lemma's fields).  To resume or extend the run into the file,
`gunzip -k structure_2_8191.jsonl.gz` first: `--out` appends to a file and skips the boards it already has.  Setup has
numbered the frames by layer since ([the frame model by
layer](../the-frame-model-by-layer-and-the-extended-problem-2026-10/README.md); [one-sided
candidates](../one-sided-candidates-in-the-search-2026-10/README.md) added `extended_frames`); the data stand for both
numberings, which are the same on every board here.

## Commits

* 631e027 (2026-10-15): the census of the frames' structure, `unzip`'s claims recorded as front and back pairs (E1).
* 8c833a1, 8b0d0d4 (2026-10-16): `build_extended` keeps the claims of `unzip` that Lemma M5⁺ allows; Theorem M⁺ against
  the oracle with the claims kept, and the mutation that skips the check.
* 3841753 (2026-10-16): `certify` searches Π⁺(O) with the claims by default, records the formulation, and the rule of
  the same problem compares whole problems.
* cd959d1, 6146627 (2026-10-16): the census checks Π⁺(∅) with the claims against the frame model (check H) and records
  the lemma's fields; the census of every board 2..8191.
* ed09ac0 (2026-10-16): the base commit the checks compared against.
* e62dd1f, 30c8504, 54dce27 (2026-10-16, 2026-10-16, 2026-10-18): the claim-by-claim branch, paused.
